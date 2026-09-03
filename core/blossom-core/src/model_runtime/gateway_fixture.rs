//! Synthetic-only Unix transport fixture for ADR-0012 checkpoint 2.
//!
//! This is not the production gateway. It accepts caller-selected socket paths
//! and identities only because it can carry crate-internal synthetic requests.

use super::{
    GatewayEventValidator, GatewayFrame, GatewayFrameDecoder, GatewayPeerCredentials,
    GatewayProfile, GatewayProtocolError, InferenceCancellation, InferenceRequest,
    ModelStreamState, NormalizedStreamEvent, ProviderStreamInput, decode_gateway_event,
    decode_gateway_hello, decode_gateway_synthetic_request, encode_gateway_event,
    encode_gateway_hello, encode_gateway_synthetic_request, validate_gateway_peer,
};
use std::collections::VecDeque;
use std::fmt;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::time::Duration;

const FIXTURE_IO_TIMEOUT: Duration = Duration::from_secs(2);
const READ_BUFFER_BYTES: usize = 8 * 1024;

/// One fixed developer-authored request for cross-crate process evidence. This
/// does not accept prompt text, model selection, intents, or a deadline.
pub fn fixed_synthetic_gateway_request() -> InferenceRequest {
    synthetic_request_for_profile(GatewayProfile::LlamaCppCpuV1)
}

fn synthetic_request_for_profile(profile: GatewayProfile) -> InferenceRequest {
    use super::{
        ConversationMessage, ConversationRole, InferenceOutputMode, InferenceRequestId,
        ModelProfile, TurnIntentCatalogue,
    };

    InferenceRequest::synthetic(
        InferenceRequestId::parse("gateway-process-1".into())
            .expect("fixed request ID must remain valid"),
        profile.provider(),
        ModelProfile::parse("fixture-model:1".into())
            .expect("fixed model profile must remain valid"),
        vec![
            ConversationMessage::new(ConversationRole::User, "synthetic".into())
                .expect("fixed message must remain valid"),
        ],
        TurnIntentCatalogue::empty(),
        InferenceOutputMode::Text,
        2_000,
    )
    .expect("fixed synthetic request must remain valid")
}

/// Route one authenticated, synthetic-only gateway request through the real
/// bounded adapter selected by the closed profile enum.
#[cfg(any(test, debug_assertions))]
pub fn serve_synthetic_gateway_via_adapter_once(
    listener: &UnixListener,
    expected_client_uid: u32,
    expected_client_gid: u32,
    profile: GatewayProfile,
    boot_id_sha256: &str,
    instance_nonce: &str,
) -> Result<(), GatewayFixtureError> {
    let (mut stream, _) = listener.accept().map_err(GatewayFixtureError::from_io)?;
    configure(&stream)?;
    let peer = GatewayPeerCredentials::from_stream(&stream)?;
    validate_gateway_peer(peer, expected_client_uid, expected_client_gid)?;
    stream
        .write_all(&encode_gateway_hello(
            profile,
            boot_id_sha256,
            instance_nonce,
        )?)
        .map_err(GatewayFixtureError::from_io)?;
    stream.flush().map_err(GatewayFixtureError::from_io)?;

    let mut reader = FrameReader::default();
    let request = decode_gateway_synthetic_request(&reader.read_one(&mut stream)?, profile)?;
    if !reader.is_idle() {
        return Err(GatewayFixtureError::UnexpectedFrame);
    }
    let cancellation = InferenceCancellation::new();
    let mut write_error = None;
    let mut emit = |event: &NormalizedStreamEvent| {
        if write_error.is_none()
            && let Err(error) = encode_gateway_event(event)
                .map_err(GatewayFixtureError::from)
                .and_then(|frame| {
                    stream
                        .write_all(&frame)
                        .map_err(GatewayFixtureError::from_io)
                })
        {
            write_error = Some(error);
        }
    };
    match profile {
        GatewayProfile::OllamaCpuV1 => super::OllamaAdapter::default()
            .stream(&request, cancellation, &mut emit)
            .map_err(GatewayFixtureError::Ollama)?,
        GatewayProfile::LlamaCppCpuV1 => super::LlamaCppAdapter::default()
            .stream(&request, cancellation, &mut emit)
            .map_err(GatewayFixtureError::LlamaCpp)?,
    }
    if let Some(error) = write_error {
        return Err(error);
    }
    stream.flush().map_err(GatewayFixtureError::from_io)?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(GatewayFixtureError::from_io)
}

pub struct SyntheticGatewayClient {
    stream: UnixStream,
    reader: FrameReader,
    profile: GatewayProfile,
}

impl SyntheticGatewayClient {
    pub fn connect_at(
        socket_path: &Path,
        expected_gateway_uid: u32,
        expected_gateway_gid: u32,
        profile: GatewayProfile,
    ) -> Result<Self, GatewayFixtureError> {
        let stream = UnixStream::connect(socket_path).map_err(GatewayFixtureError::from_io)?;
        configure(&stream)?;

        // No read or write may move before this check. The peer identity comes
        // from the same connected descriptor that will carry the request.
        let peer = GatewayPeerCredentials::from_stream(&stream)?;
        validate_gateway_peer(peer, expected_gateway_uid, expected_gateway_gid)?;

        let mut client = Self {
            stream,
            reader: FrameReader::default(),
            profile,
        };
        let hello = client.reader.read_one(&mut client.stream)?;
        decode_gateway_hello(&hello, profile)?;
        Ok(client)
    }

    pub fn infer(
        mut self,
        request: &InferenceRequest,
    ) -> Result<Vec<NormalizedStreamEvent>, GatewayFixtureError> {
        let encoded = encode_gateway_synthetic_request(request)?;
        self.stream
            .write_all(&encoded)
            .map_err(GatewayFixtureError::from_io)?;
        self.stream.flush().map_err(GatewayFixtureError::from_io)?;

        let mut validator = GatewayEventValidator::new(request.request_id());
        let mut events = Vec::new();
        while !validator.is_terminal() {
            let frame = self.reader.read_one(&mut self.stream)?;
            let event = decode_gateway_event(&frame)?;
            validator.accept(&event)?;
            events.push(event);
        }
        self.reader.require_clean_eof(&mut self.stream)?;
        Ok(events)
    }

    pub fn profile(&self) -> GatewayProfile {
        self.profile
    }
}

pub fn serve_synthetic_gateway_once(
    listener: &UnixListener,
    expected_client_uid: u32,
    expected_client_gid: u32,
    profile: GatewayProfile,
    boot_id_sha256: &str,
    instance_nonce: &str,
    response: &str,
) -> Result<(), GatewayFixtureError> {
    let (mut stream, _) = listener.accept().map_err(GatewayFixtureError::from_io)?;
    configure(&stream)?;
    let peer = GatewayPeerCredentials::from_stream(&stream)?;
    validate_gateway_peer(peer, expected_client_uid, expected_client_gid)?;

    let hello = encode_gateway_hello(profile, boot_id_sha256, instance_nonce)?;
    stream
        .write_all(&hello)
        .map_err(GatewayFixtureError::from_io)?;
    stream.flush().map_err(GatewayFixtureError::from_io)?;

    let mut reader = FrameReader::default();
    let request_frame = reader.read_one(&mut stream)?;
    if !reader.is_idle() {
        return Err(GatewayFixtureError::UnexpectedFrame);
    }
    let request = decode_gateway_synthetic_request(&request_frame, profile)?;
    let mut state = ModelStreamState::new(&request, InferenceCancellation::new());
    let events = [
        state.apply(0, ProviderStreamInput::Started)?,
        state.apply(1, ProviderStreamInput::TextDelta(response.into()))?,
        state.apply(2, ProviderStreamInput::Finished)?,
    ];
    for event in events {
        stream
            .write_all(&encode_gateway_event(&event)?)
            .map_err(GatewayFixtureError::from_io)?;
    }
    stream.flush().map_err(GatewayFixtureError::from_io)?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(GatewayFixtureError::from_io)?;
    Ok(())
}

#[derive(Default)]
struct FrameReader {
    decoder: GatewayFrameDecoder,
    pending: VecDeque<GatewayFrame>,
}

impl FrameReader {
    fn read_one(&mut self, stream: &mut UnixStream) -> Result<GatewayFrame, GatewayFixtureError> {
        if let Some(frame) = self.pending.pop_front() {
            return Ok(frame);
        }
        let mut buffer = [0; READ_BUFFER_BYTES];
        loop {
            let count = stream
                .read(&mut buffer)
                .map_err(GatewayFixtureError::from_io)?;
            if count == 0 {
                return Err(GatewayFixtureError::Disconnected);
            }
            self.pending.extend(self.decoder.push(&buffer[..count])?);
            if let Some(frame) = self.pending.pop_front() {
                return Ok(frame);
            }
        }
    }

    fn is_idle(&self) -> bool {
        self.pending.is_empty() && self.decoder.is_idle()
    }

    fn require_clean_eof(self, stream: &mut UnixStream) -> Result<(), GatewayFixtureError> {
        if !self.pending.is_empty() {
            return Err(GatewayFixtureError::UnexpectedFrame);
        }
        let mut buffer = [0; 1];
        match stream.read(&mut buffer) {
            Ok(0) => self.decoder.finish().map_err(Into::into),
            Ok(_) => Err(GatewayFixtureError::UnexpectedFrame),
            Err(error) => Err(GatewayFixtureError::from_io(error)),
        }
    }
}

fn configure(stream: &UnixStream) -> Result<(), GatewayFixtureError> {
    stream
        .set_read_timeout(Some(FIXTURE_IO_TIMEOUT))
        .map_err(GatewayFixtureError::from_io)?;
    stream
        .set_write_timeout(Some(FIXTURE_IO_TIMEOUT))
        .map_err(GatewayFixtureError::from_io)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatewayFixtureError {
    Unavailable,
    TimedOut,
    Disconnected,
    UnexpectedFrame,
    Protocol(GatewayProtocolError),
    Contract(super::ModelContractError),
    Ollama(super::OllamaAdapterError),
    LlamaCpp(super::LlamaCppAdapterError),
}

impl GatewayFixtureError {
    fn from_io(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => Self::TimedOut,
            std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::UnexpectedEof => Self::Disconnected,
            _ => Self::Unavailable,
        }
    }
}

impl From<GatewayProtocolError> for GatewayFixtureError {
    fn from(error: GatewayProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<super::ModelContractError> for GatewayFixtureError {
    fn from(error: super::ModelContractError) -> Self {
        Self::Contract(error)
    }
}

impl fmt::Display for GatewayFixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "synthetic gateway fixture is unavailable",
            Self::TimedOut => "synthetic gateway fixture timed out",
            Self::Disconnected => "synthetic gateway fixture disconnected",
            Self::UnexpectedFrame => "synthetic gateway fixture sent an unexpected frame",
            Self::Protocol(_) => "synthetic gateway fixture violated the gateway protocol",
            Self::Contract(_) => "synthetic gateway fixture violated the model contract",
            Self::Ollama(_) | Self::LlamaCpp(_) => {
                "synthetic gateway provider violated the adapter contract"
            }
        })
    }
}

impl std::error::Error for GatewayFixtureError {}

#[cfg(test)]
mod framing_tests {
    use super::*;

    #[test]
    fn reader_does_not_hide_partial_second_frame() {
        let (mut sender, mut receiver) = UnixStream::pair().unwrap();
        let hello = encode_gateway_hello(GatewayProfile::OllamaCpuV1, &"a".repeat(64), "fixture-1")
            .unwrap();
        sender.write_all(&hello).unwrap();
        sender.write_all(b"x").unwrap();
        let mut reader = FrameReader::default();
        assert_eq!(
            reader.read_one(&mut receiver).unwrap().kind(),
            super::super::GatewayMessageKind::Hello
        );
        assert!(!reader.is_idle());
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use crate::model_runtime::{
        ConversationMessage, ConversationRole, InferenceOutputMode, InferenceRequestId,
        ModelProfile, ModelProviderKind, NormalizedCompletion, NormalizedStreamKind,
        TurnIntentCatalogue,
    };
    use std::fs;
    use std::net::TcpListener;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    const CHILD_ENV: &str = "BLOSSOM_GATEWAY_FIXTURE_CHILD";
    const SOCKET_ENV: &str = "BLOSSOM_GATEWAY_FIXTURE_SOCKET";
    const MODE_ENV: &str = "BLOSSOM_GATEWAY_FIXTURE_MODE";

    fn request() -> InferenceRequest {
        InferenceRequest::synthetic(
            InferenceRequestId::parse("gateway-process-1".into()).unwrap(),
            ModelProviderKind::LlamaCpp,
            ModelProfile::parse("fixture-model:1".into()).unwrap(),
            vec![ConversationMessage::new(ConversationRole::User, "synthetic".into()).unwrap()],
            TurnIntentCatalogue::empty(),
            InferenceOutputMode::Text,
            2_000,
        )
        .unwrap()
    }

    fn socket_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "blossom-gateway-{label}-{}-{}.sock",
            std::process::id(),
            Instant::now().elapsed().as_nanos()
        ))
    }

    fn spawn_child(path: &Path, mode: &str) -> std::process::Child {
        Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("model_runtime::gateway_fixture::tests::gateway_fixture_child_process")
            .arg("--nocapture")
            .env(CHILD_ENV, "1")
            .env(SOCKET_ENV, path)
            .env(MODE_ENV, mode)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    }

    fn wait_for_socket(path: &Path) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !path.exists() {
            assert!(Instant::now() < deadline, "fixture socket was not created");
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn gateway_fixture_child_process() {
        if std::env::var_os(CHILD_ENV).is_none() {
            return;
        }
        let path = std::path::PathBuf::from(std::env::var_os(SOCKET_ENV).unwrap());
        let listener = UnixListener::bind(&path).unwrap();
        let mode = std::env::var(MODE_ENV).unwrap();
        if mode == "serve" {
            let uid = nix::unistd::geteuid().as_raw();
            let gid = nix::unistd::getegid().as_raw();
            serve_synthetic_gateway_once(
                &listener,
                uid,
                gid,
                GatewayProfile::LlamaCppCpuV1,
                &"a".repeat(64),
                "process-fixture-1",
                "fixture-ok",
            )
            .unwrap();
        } else {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut byte = [0; 1];
            assert_eq!(stream.read(&mut byte).unwrap(), 0);
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn separate_process_round_trip_validates_credentials_before_request() {
        let path = socket_path("roundtrip");
        let mut child = spawn_child(&path, "serve");
        wait_for_socket(&path);
        let uid = nix::unistd::geteuid().as_raw();
        let gid = nix::unistd::getegid().as_raw();
        let client =
            SyntheticGatewayClient::connect_at(&path, uid, gid, GatewayProfile::LlamaCppCpuV1)
                .unwrap();
        assert_eq!(client.profile(), GatewayProfile::LlamaCppCpuV1);
        let events = client.infer(&request()).unwrap();
        assert!(matches!(
            &events.last().unwrap().event,
            NormalizedStreamKind::Finished {
                completion: NormalizedCompletion::Text { content }
            } if content == "fixture-ok"
        ));
        assert!(child.wait().unwrap().success());

        let path = socket_path("wrong-peer");
        let mut child = spawn_child(&path, "expect-zero");
        wait_for_socket(&path);
        assert!(matches!(
            SyntheticGatewayClient::connect_at(
                &path,
                uid.saturating_add(1),
                gid,
                GatewayProfile::LlamaCppCpuV1,
            ),
            Err(GatewayFixtureError::Protocol(
                GatewayProtocolError::PeerCredentialMismatch
            ))
        ));
        assert!(child.wait().unwrap().success());
    }

    fn serve_http_once(
        endpoint: &str,
        content_type: &'static str,
        body: String,
    ) -> thread::JoinHandle<()> {
        let listener = TcpListener::bind(endpoint).unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            let mut expected = None;
            loop {
                let count = stream.read(&mut buffer).unwrap();
                assert_ne!(count, 0);
                request.extend_from_slice(&buffer[..count]);
                if expected.is_none()
                    && let Some(header_end) =
                        request.windows(4).position(|part| part == b"\r\n\r\n")
                {
                    let headers = std::str::from_utf8(&request[..header_end]).unwrap();
                    let content_length = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("Content-Length: "))
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();
                    expected = Some(header_end + 4 + content_length);
                }
                if expected.is_some_and(|size| request.len() >= size) {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        })
    }

    #[test]
    fn authenticated_gateway_routes_both_closed_profiles_through_real_adapters() {
        let fixtures = [
            (
                GatewayProfile::OllamaCpuV1,
                super::super::OLLAMA_ENDPOINT,
                "application/x-ndjson",
                concat!(
                    r#"{"model":"fixture-model:1","created_at":"2026-09-03T00:00:00Z","message":{"role":"assistant","content":"gateway-"},"done":false}"#,
                    "\n",
                    r#"{"model":"fixture-model:1","created_at":"2026-09-03T00:00:01Z","message":{"role":"assistant","content":"ollama"},"done":true,"done_reason":"stop","prompt_eval_count":3,"eval_count":2}"#,
                    "\n"
                )
                .to_owned(),
                "gateway-ollama",
            ),
            (
                GatewayProfile::LlamaCppCpuV1,
                super::super::LLAMA_CPP_ENDPOINT,
                "text/event-stream",
                concat!(
                    "data: ",
                    r#"{"id":"chatcmpl-gateway","object":"chat.completion.chunk","created":1,"model":"fixture-model:1","choices":[{"index":0,"delta":{"role":"assistant","content":"gateway-llama"},"finish_reason":"stop"}]}"#,
                    "\n\n",
                    "data: ",
                    r#"{"id":"chatcmpl-gateway","object":"chat.completion.chunk","created":1,"model":"fixture-model:1","choices":[],"usage":{"completion_tokens":2,"prompt_tokens":3,"total_tokens":5}}"#,
                    "\n\n",
                    "data: [DONE]\n\n"
                )
                .to_owned(),
                "gateway-llama",
            ),
        ];
        for (profile, endpoint, content_type, body, expected) in fixtures {
            let provider = serve_http_once(endpoint, content_type, body);
            let socket = socket_path("adapter");
            let listener = UnixListener::bind(&socket).unwrap();
            let uid = nix::unistd::geteuid().as_raw();
            let gid = nix::unistd::getegid().as_raw();
            let gateway = thread::spawn(move || {
                serve_synthetic_gateway_via_adapter_once(
                    &listener,
                    uid,
                    gid,
                    profile,
                    &"a".repeat(64),
                    "adapter-fixture-1",
                )
            });
            let client = SyntheticGatewayClient::connect_at(&socket, uid, gid, profile).unwrap();
            let events = client
                .infer(&synthetic_request_for_profile(profile))
                .unwrap();
            assert!(matches!(
                &events.last().unwrap().event,
                NormalizedStreamKind::Finished {
                    completion: NormalizedCompletion::Text { content }
                } if content == expected
            ));
            assert_eq!(gateway.join().unwrap(), Ok(()));
            provider.join().unwrap();
            fs::remove_file(socket).unwrap();
        }
    }
}
