//! Authenticated client for the sole production private-inference gateway.

use super::{
    ConversationMessage, GatewayEventValidator, GatewayFrame, GatewayFrameDecoder,
    GatewayPeerCredentials, GatewayProfile, InferenceOutputMode, InferenceRequestId,
    NormalizedCompletion, NormalizedStreamKind, TurnIntentCatalogue, decode_gateway_event,
    decode_gateway_hello, encode_gateway_private_request, validate_gateway_peer,
};
use std::collections::VecDeque;
use std::fmt;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

const IO_TIMEOUT: Duration = Duration::from_secs(30);
const READ_BUFFER_BYTES: usize = 8 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrivateGatewayClientError {
    ConnectionUnavailable,
    UnexpectedGatewayIdentity,
    Protocol,
    InferenceFailed,
}

impl fmt::Display for PrivateGatewayClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ConnectionUnavailable => "private model gateway is unavailable",
            Self::UnexpectedGatewayIdentity => "private model gateway identity is unexpected",
            Self::Protocol => "private model gateway protocol failed closed",
            Self::InferenceFailed => "private model inference did not complete",
        })
    }
}

impl std::error::Error for PrivateGatewayClientError {}

pub struct PrivateGatewayClient {
    stream: UnixStream,
    reader: FrameReader,
}

impl PrivateGatewayClient {
    /// Connect and authenticate the peer on the same descriptor before any
    /// prompt bytes are written.
    pub fn connect_at(
        socket_path: &Path,
        expected_gateway_uid: u32,
        expected_gateway_gid: u32,
        profile: GatewayProfile,
    ) -> Result<Self, PrivateGatewayClientError> {
        Self::connect_at_profiles(
            socket_path,
            expected_gateway_uid,
            expected_gateway_gid,
            &[profile],
        )
    }

    /// Authenticate one gateway whose hello names exactly one member of the
    /// closed, caller-compiled profile set.
    pub fn connect_at_profiles(
        socket_path: &Path,
        expected_gateway_uid: u32,
        expected_gateway_gid: u32,
        profiles: &[GatewayProfile],
    ) -> Result<Self, PrivateGatewayClientError> {
        if profiles.is_empty() {
            return Err(PrivateGatewayClientError::Protocol);
        }
        let stream = UnixStream::connect(socket_path)
            .map_err(|_| PrivateGatewayClientError::ConnectionUnavailable)?;
        stream
            .set_read_timeout(Some(IO_TIMEOUT))
            .map_err(|_| PrivateGatewayClientError::ConnectionUnavailable)?;
        stream
            .set_write_timeout(Some(IO_TIMEOUT))
            .map_err(|_| PrivateGatewayClientError::ConnectionUnavailable)?;
        let peer = GatewayPeerCredentials::from_stream(&stream)
            .map_err(|_| PrivateGatewayClientError::UnexpectedGatewayIdentity)?;
        validate_gateway_peer(peer, expected_gateway_uid, expected_gateway_gid)
            .map_err(|_| PrivateGatewayClientError::UnexpectedGatewayIdentity)?;
        let mut client = Self {
            stream,
            reader: FrameReader::default(),
        };
        let hello = client.reader.read_one(&mut client.stream)?;
        if !profiles
            .iter()
            .copied()
            .any(|profile| decode_gateway_hello(&hello, profile).is_ok())
        {
            return Err(PrivateGatewayClientError::Protocol);
        }
        Ok(client)
    }

    pub fn infer(
        mut self,
        request_id: &InferenceRequestId,
        messages: &[ConversationMessage],
        intents: &TurnIntentCatalogue,
        deadline_ms: u64,
    ) -> Result<NormalizedCompletion, PrivateGatewayClientError> {
        let frame = encode_gateway_private_request(
            request_id,
            messages,
            intents,
            InferenceOutputMode::BlossomTurn,
            deadline_ms,
        )
        .map_err(|_| PrivateGatewayClientError::Protocol)?;
        self.stream
            .write_all(&frame)
            .and_then(|_| self.stream.flush())
            .map_err(|_| PrivateGatewayClientError::ConnectionUnavailable)?;
        let mut validator = GatewayEventValidator::new(request_id);
        loop {
            let event = decode_gateway_event(&self.reader.read_one(&mut self.stream)?)
                .map_err(|_| PrivateGatewayClientError::Protocol)?;
            validator
                .accept(&event)
                .map_err(|_| PrivateGatewayClientError::Protocol)?;
            match event.event {
                NormalizedStreamKind::Finished { completion } => return Ok(completion),
                NormalizedStreamKind::Cancelled | NormalizedStreamKind::Failed { .. } => {
                    return Err(PrivateGatewayClientError::InferenceFailed);
                }
                _ => {}
            }
        }
    }
}

#[derive(Default)]
struct FrameReader {
    decoder: GatewayFrameDecoder,
    pending: VecDeque<GatewayFrame>,
}

impl FrameReader {
    fn read_one(
        &mut self,
        stream: &mut UnixStream,
    ) -> Result<GatewayFrame, PrivateGatewayClientError> {
        if let Some(frame) = self.pending.pop_front() {
            return Ok(frame);
        }
        let mut bytes = [0_u8; READ_BUFFER_BYTES];
        loop {
            let count = stream
                .read(&mut bytes)
                .map_err(|_| PrivateGatewayClientError::ConnectionUnavailable)?;
            if count == 0 {
                return Err(PrivateGatewayClientError::ConnectionUnavailable);
            }
            self.pending.extend(
                self.decoder
                    .push(&bytes[..count])
                    .map_err(|_| PrivateGatewayClientError::Protocol)?,
            );
            if let Some(frame) = self.pending.pop_front() {
                return Ok(frame);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "linux")]
    use std::os::unix::net::UnixListener;
    #[cfg(target_os = "linux")]
    use std::sync::mpsc;
    #[cfg(target_os = "linux")]
    use std::thread;

    #[test]
    fn missing_gateway_fails_closed() {
        let path = std::env::temp_dir().join(format!(
            "blossom-missing-gateway-{}.sock",
            std::process::id()
        ));
        assert_eq!(
            PrivateGatewayClient::connect_at(&path, 1, 1, GatewayProfile::LlamaCppCpuV1).err(),
            Some(PrivateGatewayClientError::ConnectionUnavailable)
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn unexpected_gateway_owner_fails_before_sending_request_bytes() {
        let path =
            std::env::temp_dir().join(format!("blossom-wrong-gateway-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();
        let (sender, receiver) = mpsc::channel();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_millis(100)))
                .unwrap();
            let mut byte = [0_u8; 1];
            sender.send(stream.read(&mut byte).unwrap_or(0)).unwrap();
        });
        let actual_uid = nix::unistd::geteuid().as_raw();
        let actual_gid = nix::unistd::getegid().as_raw();
        assert_eq!(
            PrivateGatewayClient::connect_at(
                &path,
                actual_uid.saturating_add(1),
                actual_gid,
                GatewayProfile::LlamaCppCpuV1,
            )
            .err(),
            Some(PrivateGatewayClientError::UnexpectedGatewayIdentity)
        );
        assert_eq!(receiver.recv().unwrap(), 0);
        server.join().unwrap();
        let _ = std::fs::remove_file(path);
    }
}
