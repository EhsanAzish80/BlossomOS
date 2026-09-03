#![forbid(unsafe_code)]

use blossom_core::{
    ConversationMessage, ConversationRole, GatewayEventValidator, GatewayFrame,
    GatewayFrameDecoder, GatewayProfile, InferenceOutputMode, InferenceRequestId,
    NormalizedStreamKind, TurnIntentCatalogue, decode_gateway_event, decode_gateway_hello,
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
    while !validator.is_terminal() {
        let event =
            decode_gateway_event(&reader.read_one(&mut stream)?).map_err(|_| "invalid event")?;
        validator.accept(&event).map_err(|_| "invalid sequence")?;
        if let NormalizedStreamKind::Finished { completion } = event.event {
            completed_text = matches!(
                completion,
                blossom_core::NormalizedCompletion::Text { ref content } if !content.is_empty()
            );
        }
    }
    if completed_text {
        Ok(())
    } else {
        Err("inference did not complete with validated text".into())
    }
}

fn main() {
    let result = match std::env::args().nth(1).as_deref() {
        Some("expect-rejected") => expect_rejected(),
        Some("infer") => infer(),
        _ => Err("expected expect-rejected or infer".into()),
    };
    if let Err(error) = result {
        eprintln!("installed gateway probe failed: {error}");
        std::process::exit(1);
    }
}
