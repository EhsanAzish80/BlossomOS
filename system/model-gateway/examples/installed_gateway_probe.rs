#![forbid(unsafe_code)]

use blossom_core::{
    ConversationMessage, ConversationRole, GatewayEventValidator, GatewayFrame,
    GatewayFrameDecoder, GatewayProfile, InferenceOutputMode, InferenceRequestId,
    NormalizedCompletion, NormalizedStreamKind, ProviderFailureCategory, TurnIntentCatalogue,
    decode_gateway_event, decode_gateway_hello, encode_gateway_cancel,
    encode_gateway_private_request,
};
use blossom_model_gateway::PRODUCTION_SOCKET_PATH;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

struct Reader {
    decoder: GatewayFrameDecoder,
    pending: VecDeque<GatewayFrame>,
}

impl Reader {
    fn new() -> Self {
        Self {
            decoder: GatewayFrameDecoder::default(),
            pending: VecDeque::new(),
        }
    }

    fn read_one(&mut self, stream: &mut UnixStream) -> Result<GatewayFrame, String> {
        if let Some(frame) = self.pending.pop_front() {
            return Ok(frame);
        }
        let mut bytes = [0_u8; 8 * 1024];
        loop {
            let count = stream.read(&mut bytes).map_err(|_| "read failed")?;
            if count == 0 {
                return Err("unexpected eof".into());
            }
            self.pending.extend(
                self.decoder
                    .push(&bytes[..count])
                    .map_err(|_| "bad frame")?,
            );
            if let Some(frame) = self.pending.pop_front() {
                return Ok(frame);
            }
        }
    }
}

fn expect_rejected() -> Result<(), String> {
    let mut stream = match UnixStream::connect(PRODUCTION_SOCKET_PATH) {
        Ok(stream) => stream,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => return Ok(()),
        Err(_) => return Err("connection failed unexpectedly".into()),
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| "timeout setup failed")?;
    let mut byte = [0_u8; 1];
    match stream.read(&mut byte) {
        Ok(0) => Ok(()),
        Ok(_) => Err("rejected client received gateway bytes".into()),
        Err(_) => Err("rejection was not observable".into()),
    }
}

fn exhaust_audit() -> Result<(), String> {
    const MIN_REJECTIONS: usize = 1_000;
    const MAX_REJECTIONS: usize = 5_000;
    const STOPPED_RETRIES: usize = 100;

    let mut completed = 0_usize;
    let mut unavailable = 0_usize;
    while completed < MAX_REJECTIONS {
        let mut stream = match UnixStream::connect(PRODUCTION_SOCKET_PATH) {
            Ok(stream) => {
                unavailable = 0;
                stream
            }
            Err(_) => {
                unavailable += 1;
                if unavailable == STOPPED_RETRIES {
                    return if completed >= MIN_REJECTIONS {
                        Ok(())
                    } else {
                        Err("gateway stopped before capacity evidence".into())
                    };
                }
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|_| "timeout setup failed")?;
        let mut byte = [0_u8; 1];
        match stream.read(&mut byte) {
            Ok(0) => completed += 1,
            Ok(_) => return Err("rejected client received gateway bytes".into()),
            Err(_) => return Err("rejection was not observable".into()),
        }
    }
    Err("gateway exceeded the closed audit capacity".into())
}

fn infer() -> Result<(), String> {
    let mut stream = UnixStream::connect(PRODUCTION_SOCKET_PATH).map_err(|_| "connect failed")?;
    stream
        .set_read_timeout(Some(Duration::from_secs(130)))
        .map_err(|_| "timeout setup failed")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| "timeout setup failed")?;
    let mut reader = Reader::new();
    let hello = reader.read_one(&mut stream)?;
    decode_gateway_hello(&hello, GatewayProfile::LlamaCppCpuV1).map_err(|_| "invalid hello")?;
    let request_id = InferenceRequestId::parse("installed-evidence-1".into())
        .map_err(|_| "request id rejected")?;
    let messages = [ConversationMessage::new(
        ConversationRole::User,
        "Reply with one short greeting.".into(),
    )
    .map_err(|_| "message rejected")?];
    let request = encode_gateway_private_request(
        &request_id,
        &messages,
        &TurnIntentCatalogue::empty(),
        InferenceOutputMode::Text,
        120_000,
    )
    .map_err(|_| "request encoding failed")?;
    stream
        .write_all(&request)
        .map_err(|_| "request write failed")?;
    let mut validator = GatewayEventValidator::new(&request_id);
    let mut completed_text = false;
    let mut terminal_category = "missing";
    while !validator.is_terminal() {
        let event =
            decode_gateway_event(&reader.read_one(&mut stream)?).map_err(|_| "invalid event")?;
        validator.accept(&event).map_err(|_| "invalid sequence")?;
        match &event.event {
            NormalizedStreamKind::Finished {
                completion: NormalizedCompletion::Text { content },
            } if !content.is_empty() => {
                completed_text = true;
                terminal_category = "completed_text";
            }
            NormalizedStreamKind::Finished {
                completion: NormalizedCompletion::ToolIntents { .. },
            } => terminal_category = "completed_tool_intents",
            NormalizedStreamKind::Cancelled => terminal_category = "cancelled",
            NormalizedStreamKind::Failed { category } => {
                terminal_category = match category {
                    ProviderFailureCategory::Unavailable => "failed_unavailable",
                    ProviderFailureCategory::TimedOut => "failed_timed_out",
                    ProviderFailureCategory::Disconnected => "failed_disconnected",
                    ProviderFailureCategory::Malformed => "failed_malformed",
                    ProviderFailureCategory::ProviderFailed => "failed_provider",
                    ProviderFailureCategory::OutputLimit => "failed_output_limit",
                };
            }
            _ => {}
        }
    }
    if completed_text {
        Ok(())
    } else {
        Err(format!("inference terminal category: {terminal_category}"))
    }
}

fn cancel() -> Result<(), String> {
    let mut stream = UnixStream::connect(PRODUCTION_SOCKET_PATH).map_err(|_| "connect failed")?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|_| "timeout setup failed")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| "timeout setup failed")?;
    let mut reader = Reader::new();
    let hello = reader.read_one(&mut stream)?;
    decode_gateway_hello(&hello, GatewayProfile::LlamaCppCpuV1).map_err(|_| "invalid hello")?;
    let request_id = InferenceRequestId::parse("installed-cancel-1".into())
        .map_err(|_| "request id rejected")?;
    let messages = [ConversationMessage::new(
        ConversationRole::User,
        "Write the numbers from one to one hundred, one per line.".into(),
    )
    .map_err(|_| "message rejected")?];
    let request = encode_gateway_private_request(
        &request_id,
        &messages,
        &TurnIntentCatalogue::empty(),
        InferenceOutputMode::Text,
        25_000,
    )
    .map_err(|_| "request encoding failed")?;
    stream
        .write_all(&request)
        .map_err(|_| "request write failed")?;

    let mut validator = GatewayEventValidator::new(&request_id);
    let started =
        decode_gateway_event(&reader.read_one(&mut stream)?).map_err(|_| "invalid event")?;
    validator.accept(&started).map_err(|_| "invalid sequence")?;
    if !matches!(started.event, NormalizedStreamKind::Started) {
        return Err("request did not start before cancellation".into());
    }

    let cancellation = encode_gateway_cancel(&request_id).map_err(|_| "cancel encoding failed")?;
    stream
        .write_all(&cancellation)
        .map_err(|_| "cancel write failed")?;

    while !validator.is_terminal() {
        let event =
            decode_gateway_event(&reader.read_one(&mut stream)?).map_err(|_| "invalid event")?;
        validator.accept(&event).map_err(|_| "invalid sequence")?;
        match event.event {
            NormalizedStreamKind::Cancelled => return Ok(()),
            NormalizedStreamKind::Finished { .. } => {
                return Err("request completed after cancellation".into());
            }
            NormalizedStreamKind::Failed { .. } => {
                return Err("request failed instead of cancelling".into());
            }
            _ => {}
        }
    }
    Err("request ended without cancellation".into())
}

fn main() {
    let result = match std::env::args().nth(1).as_deref() {
        Some("expect-rejected") => expect_rejected(),
        Some("exhaust-audit") => exhaust_audit(),
        Some("infer") => infer(),
        Some("cancel") => cancel(),
        _ => Err("expected expect-rejected, exhaust-audit, infer, or cancel".into()),
    };
    if let Err(error) = result {
        eprintln!("installed gateway probe failed: {error}");
        std::process::exit(1);
    }
}
