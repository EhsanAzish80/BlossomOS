//! Closed framing and peer-credential primitives for ADR-0012 checkpoint 1.
//!
//! This module does not create a listener, install a service, accept private
//! input, or contact a provider.

use super::{
    ConversationMessage, ConversationRole, InferenceOutputMode, InferenceRequest,
    InferenceRequestId, MAX_INFERENCE_REQUEST_BYTES, MAX_MESSAGES, MAX_OUTPUT_BYTES,
    MAX_TEXT_DELTA_BYTES, MAX_TOOL_INTENTS, MODEL_PROTOCOL_VERSION, ModelIntentKind, ModelProfile,
    ModelProviderKind, NormalizedCompletion, NormalizedStreamEvent, NormalizedStreamKind,
    TurnIntentCatalogue,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;

pub const GATEWAY_PROTOCOL_VERSION: u16 = 1;
pub const MAX_GATEWAY_FRAME_BYTES: usize = MAX_INFERENCE_REQUEST_BYTES;
const MAGIC: &[u8; 8] = b"BLSMGW01";
const HEADER_BYTES: usize = 48;
const SHA256_BYTES: usize = 32;
const MAX_GATEWAY_BUFFER_BYTES: usize = HEADER_BYTES + MAX_GATEWAY_FRAME_BYTES;
const MAX_INSTANCE_NONCE_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum GatewayMessageKind {
    Hello = 1,
    SyntheticInference = 2,
    Cancel = 3,
    Event = 4,
    PrivateInference = 5,
}

impl GatewayMessageKind {
    fn parse(value: u8) -> Result<Self, GatewayProtocolError> {
        match value {
            1 => Ok(Self::Hello),
            2 => Ok(Self::SyntheticInference),
            3 => Ok(Self::Cancel),
            4 => Ok(Self::Event),
            5 => Ok(Self::PrivateInference),
            _ => Err(GatewayProtocolError::UnknownMessageKind),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GatewayFrame {
    kind: GatewayMessageKind,
    payload: Vec<u8>,
}

impl GatewayFrame {
    pub fn kind(&self) -> GatewayMessageKind {
        self.kind
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    fn encode(kind: GatewayMessageKind, payload: Vec<u8>) -> Result<Vec<u8>, GatewayProtocolError> {
        if payload.is_empty() || payload.len() > MAX_GATEWAY_FRAME_BYTES {
            return Err(GatewayProtocolError::FrameTooLarge);
        }
        let length =
            u32::try_from(payload.len()).map_err(|_| GatewayProtocolError::FrameTooLarge)?;
        let digest = Sha256::digest(&payload);
        let mut frame = Vec::with_capacity(HEADER_BYTES + payload.len());
        frame.extend_from_slice(MAGIC);
        frame.extend_from_slice(&GATEWAY_PROTOCOL_VERSION.to_be_bytes());
        frame.push(kind as u8);
        frame.push(0);
        frame.extend_from_slice(&length.to_be_bytes());
        frame.extend_from_slice(&digest);
        frame.extend_from_slice(&payload);
        Ok(frame)
    }
}

#[derive(Default)]
pub struct GatewayFrameDecoder {
    buffer: Vec<u8>,
    failed: bool,
}

impl GatewayFrameDecoder {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<GatewayFrame>, GatewayProtocolError> {
        if self.failed {
            return Err(GatewayProtocolError::DecoderPoisoned);
        }
        if self.buffer.len().saturating_add(bytes.len()) > MAX_GATEWAY_BUFFER_BYTES {
            self.failed = true;
            return Err(GatewayProtocolError::FrameTooLarge);
        }
        self.buffer.extend_from_slice(bytes);
        let result = self.decode_available();
        if result.is_err() {
            self.failed = true;
            self.buffer.clear();
        }
        result
    }

    pub fn finish(mut self) -> Result<(), GatewayProtocolError> {
        if self.failed {
            return Err(GatewayProtocolError::DecoderPoisoned);
        }
        if !self.decode_available()?.is_empty() || !self.buffer.is_empty() {
            return Err(GatewayProtocolError::TruncatedFrame);
        }
        Ok(())
    }

    pub fn is_idle(&self) -> bool {
        self.buffer.is_empty() && !self.failed
    }

    fn decode_available(&mut self) -> Result<Vec<GatewayFrame>, GatewayProtocolError> {
        let mut frames = Vec::new();
        while self.buffer.len() >= HEADER_BYTES {
            if &self.buffer[..MAGIC.len()] != MAGIC {
                return Err(GatewayProtocolError::InvalidMagic);
            }
            if u16::from_be_bytes([self.buffer[8], self.buffer[9]]) != GATEWAY_PROTOCOL_VERSION {
                return Err(GatewayProtocolError::UnsupportedVersion);
            }
            let kind = GatewayMessageKind::parse(self.buffer[10])?;
            if self.buffer[11] != 0 {
                return Err(GatewayProtocolError::InvalidFlags);
            }
            let length = u32::from_be_bytes([
                self.buffer[12],
                self.buffer[13],
                self.buffer[14],
                self.buffer[15],
            ]) as usize;
            if length == 0 || length > MAX_GATEWAY_FRAME_BYTES {
                return Err(GatewayProtocolError::FrameTooLarge);
            }
            let total = HEADER_BYTES
                .checked_add(length)
                .ok_or(GatewayProtocolError::FrameTooLarge)?;
            if self.buffer.len() < total {
                break;
            }
            let payload = &self.buffer[HEADER_BYTES..total];
            if Sha256::digest(payload).as_slice() != &self.buffer[16..16 + SHA256_BYTES] {
                return Err(GatewayProtocolError::DigestMismatch);
            }
            frames.push(GatewayFrame {
                kind,
                payload: payload.to_vec(),
            });
            self.buffer.drain(..total);
        }
        Ok(frames)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayProfile {
    OllamaCpuV1,
    LlamaCppCpuV1,
}

impl GatewayProfile {
    pub(super) fn provider(self) -> ModelProviderKind {
        match self {
            Self::OllamaCpuV1 => ModelProviderKind::Ollama,
            Self::LlamaCppCpuV1 => ModelProviderKind::LlamaCpp,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HelloPayload {
    protocol_version: u16,
    model_protocol_version: u16,
    profile: GatewayProfile,
    boot_id_sha256: String,
    instance_nonce: String,
}

pub fn encode_gateway_hello(
    profile: GatewayProfile,
    boot_id_sha256: &str,
    instance_nonce: &str,
) -> Result<Vec<u8>, GatewayProtocolError> {
    validate_digest(boot_id_sha256)?;
    validate_nonce(instance_nonce)?;
    encode_json(
        GatewayMessageKind::Hello,
        &HelloPayload {
            protocol_version: GATEWAY_PROTOCOL_VERSION,
            model_protocol_version: MODEL_PROTOCOL_VERSION,
            profile,
            boot_id_sha256: boot_id_sha256.into(),
            instance_nonce: instance_nonce.into(),
        },
    )
}

pub fn decode_gateway_hello(
    frame: &GatewayFrame,
    expected_profile: GatewayProfile,
) -> Result<(String, String), GatewayProtocolError> {
    require_kind(frame, GatewayMessageKind::Hello)?;
    let payload: HelloPayload = decode_json(&frame.payload)?;
    if payload.protocol_version != GATEWAY_PROTOCOL_VERSION
        || payload.model_protocol_version != MODEL_PROTOCOL_VERSION
        || payload.profile != expected_profile
    {
        return Err(GatewayProtocolError::IdentityMismatch);
    }
    validate_digest(&payload.boot_id_sha256)?;
    validate_nonce(&payload.instance_nonce)?;
    require_canonical(&frame.payload, &payload)?;
    Ok((payload.boot_id_sha256, payload.instance_nonce))
}

pub fn encode_gateway_synthetic_request(
    request: &InferenceRequest,
) -> Result<Vec<u8>, GatewayProtocolError> {
    request
        .validate()
        .map_err(|_| GatewayProtocolError::InvalidRequest)?;
    let payload = serde_json::to_vec(request).map_err(|_| GatewayProtocolError::EncodingFailed)?;
    GatewayFrame::encode(GatewayMessageKind::SyntheticInference, payload)
}

pub fn decode_gateway_synthetic_request(
    frame: &GatewayFrame,
    expected_profile: GatewayProfile,
) -> Result<InferenceRequest, GatewayProtocolError> {
    require_kind(frame, GatewayMessageKind::SyntheticInference)?;
    let wire: WireInferenceRequest = decode_json(&frame.payload)?;
    if wire.provider != expected_profile.provider() {
        return Err(GatewayProtocolError::IdentityMismatch);
    }
    wire.into_request(&frame.payload)
}

/// Decode an authority-free production payload and inject provider, model and
/// private classification from the already admitted code-owned profile.
pub fn decode_gateway_private_request(
    frame: &GatewayFrame,
    provider: ModelProviderKind,
    model: ModelProfile,
) -> Result<InferenceRequest, GatewayProtocolError> {
    require_kind(frame, GatewayMessageKind::PrivateInference)?;
    let wire: WirePrivateInferenceRequest = decode_json(&frame.payload)?;
    wire.into_request(&frame.payload, provider, model)
}

/// Encode the authority-free client payload. Provider identity and private
/// classification cannot be supplied through this API or its wire schema.
pub fn encode_gateway_private_request(
    request_id: &InferenceRequestId,
    messages: &[ConversationMessage],
    intents: &TurnIntentCatalogue,
    output_mode: InferenceOutputMode,
    deadline_ms: u64,
) -> Result<Vec<u8>, GatewayProtocolError> {
    let wire = WirePrivateInferenceRequest {
        version: MODEL_PROTOCOL_VERSION,
        request_id: request_id.as_str().into(),
        messages: messages
            .iter()
            .map(|message| WireMessage {
                role: match message.role {
                    ConversationRole::System => WireRole::System,
                    ConversationRole::User => WireRole::User,
                    ConversationRole::Assistant => WireRole::Assistant,
                    ConversationRole::Tool => WireRole::Tool,
                },
                content: message.content.clone(),
            })
            .collect(),
        intents: WireCatalogue {
            eligible: intents.eligible.clone(),
        },
        output_mode: match output_mode {
            InferenceOutputMode::Text => WireOutputMode::Text,
            InferenceOutputMode::BlossomTurn => WireOutputMode::BlossomTurn,
        },
        deadline_ms,
    };
    let payload = serde_json::to_vec(&wire).map_err(|_| GatewayProtocolError::EncodingFailed)?;
    // Reconstruct through the same validator used by the server. The fixed
    // provider/model values are validation placeholders and are never encoded.
    WirePrivateInferenceRequest::into_request(
        wire,
        &payload,
        ModelProviderKind::LlamaCpp,
        ModelProfile::parse("validation-only".into())
            .map_err(|_| GatewayProtocolError::InvalidRequest)?,
    )?;
    GatewayFrame::encode(GatewayMessageKind::PrivateInference, payload)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WirePrivateInferenceRequest {
    version: u16,
    request_id: String,
    messages: Vec<WireMessage>,
    intents: WireCatalogue,
    output_mode: WireOutputMode,
    deadline_ms: u64,
}

impl WirePrivateInferenceRequest {
    fn into_request(
        self,
        original: &[u8],
        provider: ModelProviderKind,
        model: ModelProfile,
    ) -> Result<InferenceRequest, GatewayProtocolError> {
        if self.version != MODEL_PROTOCOL_VERSION
            || self.messages.is_empty()
            || self.messages.len() > MAX_MESSAGES
        {
            return Err(GatewayProtocolError::InvalidRequest);
        }
        let canonical =
            serde_json::to_vec(&self).map_err(|_| GatewayProtocolError::EncodingFailed)?;
        if canonical != original {
            return Err(GatewayProtocolError::NonCanonicalPayload);
        }
        let messages = decode_wire_messages(self.messages)?;
        let intents = TurnIntentCatalogue::from_eligible(self.intents.eligible)
            .map_err(|_| GatewayProtocolError::InvalidRequest)?;
        InferenceRequest::private(
            InferenceRequestId::parse(self.request_id)
                .map_err(|_| GatewayProtocolError::InvalidRequest)?,
            provider,
            model,
            messages,
            intents,
            decode_output_mode(self.output_mode),
            self.deadline_ms,
        )
        .map_err(|_| GatewayProtocolError::InvalidRequest)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireInferenceRequest {
    version: u16,
    request_id: String,
    provider: ModelProviderKind,
    model: String,
    input_classification: WireInputClassification,
    messages: Vec<WireMessage>,
    intents: WireCatalogue,
    output_mode: WireOutputMode,
    deadline_ms: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireInputClassification {
    Synthetic,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireMessage {
    role: WireRole,
    content: String,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireCatalogue {
    eligible: BTreeSet<ModelIntentKind>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireOutputMode {
    Text,
    BlossomTurn,
}

impl WireInferenceRequest {
    fn into_request(self, original: &[u8]) -> Result<InferenceRequest, GatewayProtocolError> {
        if self.version != MODEL_PROTOCOL_VERSION
            || !matches!(
                self.input_classification,
                WireInputClassification::Synthetic
            )
            || self.messages.is_empty()
            || self.messages.len() > MAX_MESSAGES
        {
            return Err(GatewayProtocolError::InvalidRequest);
        }
        let messages = decode_wire_messages(self.messages)?;
        let intents = TurnIntentCatalogue::from_eligible(self.intents.eligible)
            .map_err(|_| GatewayProtocolError::InvalidRequest)?;
        let output_mode = decode_output_mode(self.output_mode);
        let request = InferenceRequest::synthetic(
            InferenceRequestId::parse(self.request_id)
                .map_err(|_| GatewayProtocolError::InvalidRequest)?,
            self.provider,
            ModelProfile::parse(self.model).map_err(|_| GatewayProtocolError::InvalidRequest)?,
            messages,
            intents,
            output_mode,
            self.deadline_ms,
        )
        .map_err(|_| GatewayProtocolError::InvalidRequest)?;
        let canonical =
            serde_json::to_vec(&request).map_err(|_| GatewayProtocolError::EncodingFailed)?;
        if canonical != original {
            return Err(GatewayProtocolError::NonCanonicalPayload);
        }
        Ok(request)
    }
}

fn decode_wire_messages(
    messages: Vec<WireMessage>,
) -> Result<Vec<ConversationMessage>, GatewayProtocolError> {
    messages
        .into_iter()
        .map(|message| {
            let role = match message.role {
                WireRole::System => ConversationRole::System,
                WireRole::User => ConversationRole::User,
                WireRole::Assistant => ConversationRole::Assistant,
                WireRole::Tool => ConversationRole::Tool,
            };
            ConversationMessage::new(role, message.content)
                .map_err(|_| GatewayProtocolError::InvalidRequest)
        })
        .collect()
}

fn decode_output_mode(mode: WireOutputMode) -> InferenceOutputMode {
    match mode {
        WireOutputMode::Text => InferenceOutputMode::Text,
        WireOutputMode::BlossomTurn => InferenceOutputMode::BlossomTurn,
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CancelPayload {
    request_id: String,
}

pub fn encode_gateway_cancel(
    request_id: &InferenceRequestId,
) -> Result<Vec<u8>, GatewayProtocolError> {
    encode_json(
        GatewayMessageKind::Cancel,
        &CancelPayload {
            request_id: request_id.as_str().into(),
        },
    )
}

pub fn decode_gateway_cancel(
    frame: &GatewayFrame,
) -> Result<InferenceRequestId, GatewayProtocolError> {
    require_kind(frame, GatewayMessageKind::Cancel)?;
    let payload: CancelPayload = decode_json(&frame.payload)?;
    require_canonical(&frame.payload, &payload)?;
    InferenceRequestId::parse(payload.request_id).map_err(|_| GatewayProtocolError::InvalidRequest)
}

pub fn encode_gateway_event(
    event: &NormalizedStreamEvent,
) -> Result<Vec<u8>, GatewayProtocolError> {
    validate_event_shape(event)?;
    encode_json(GatewayMessageKind::Event, event)
}

pub fn decode_gateway_event(
    frame: &GatewayFrame,
) -> Result<NormalizedStreamEvent, GatewayProtocolError> {
    require_kind(frame, GatewayMessageKind::Event)?;
    let event: NormalizedStreamEvent = decode_json(&frame.payload)?;
    validate_event_shape(&event)?;
    require_canonical(&frame.payload, &event)?;
    Ok(event)
}

fn validate_event_shape(event: &NormalizedStreamEvent) -> Result<(), GatewayProtocolError> {
    InferenceRequestId::parse(event.request_id.clone())
        .map_err(|_| GatewayProtocolError::InvalidEvent)?;
    if event.version != MODEL_PROTOCOL_VERSION {
        return Err(GatewayProtocolError::InvalidEvent);
    }
    match &event.event {
        NormalizedStreamKind::TextDelta { content }
            if content.is_empty()
                || content.len() > MAX_TEXT_DELTA_BYTES
                || content.contains('\0') =>
        {
            Err(GatewayProtocolError::InvalidEvent)
        }
        NormalizedStreamKind::ToolIntents { intents }
        | NormalizedStreamKind::Finished {
            completion: NormalizedCompletion::ToolIntents { intents },
        } => validate_intents(intents),
        NormalizedStreamKind::Finished {
            completion: NormalizedCompletion::Text { content },
        } if content.is_empty() || content.len() > MAX_OUTPUT_BYTES || content.contains('\0') => {
            Err(GatewayProtocolError::InvalidEvent)
        }
        _ => Ok(()),
    }
}

fn validate_intents(intents: &[super::ProposedToolIntent]) -> Result<(), GatewayProtocolError> {
    if intents.is_empty() || intents.len() > MAX_TOOL_INTENTS {
        return Err(GatewayProtocolError::InvalidEvent);
    }
    let unique: BTreeSet<_> = intents.iter().map(|intent| intent.kind()).collect();
    if unique.len() != intents.len() {
        return Err(GatewayProtocolError::InvalidEvent);
    }
    Ok(())
}

pub struct GatewayEventValidator {
    request_id: String,
    next_sequence: u64,
    started: bool,
    terminal: bool,
    output: EventOutput,
    usage_seen: bool,
}

#[derive(Default)]
enum EventOutput {
    #[default]
    None,
    Text(String),
    Intents(Vec<super::ProposedToolIntent>),
}

impl GatewayEventValidator {
    pub fn new(request_id: &InferenceRequestId) -> Self {
        Self {
            request_id: request_id.as_str().into(),
            next_sequence: 0,
            started: false,
            terminal: false,
            output: EventOutput::None,
            usage_seen: false,
        }
    }

    pub fn accept(&mut self, event: &NormalizedStreamEvent) -> Result<(), GatewayProtocolError> {
        validate_event_shape(event)?;
        if self.terminal
            || event.request_id != self.request_id
            || event.sequence != self.next_sequence
        {
            return Err(GatewayProtocolError::InvalidSequence);
        }
        match &event.event {
            NormalizedStreamKind::Started if !self.started && event.sequence == 0 => {
                self.started = true;
            }
            NormalizedStreamKind::Cancelled if !self.started && event.sequence == 0 => {
                self.terminal = true;
            }
            _ if !self.started => return Err(GatewayProtocolError::InvalidSequence),
            NormalizedStreamKind::Started => return Err(GatewayProtocolError::InvalidSequence),
            NormalizedStreamKind::TextDelta { content } => match &mut self.output {
                EventOutput::None => self.output = EventOutput::Text(content.clone()),
                EventOutput::Text(accumulated) => {
                    if accumulated.len().saturating_add(content.len()) > MAX_OUTPUT_BYTES {
                        return Err(GatewayProtocolError::InvalidEvent);
                    }
                    accumulated.push_str(content);
                }
                EventOutput::Intents(_) => return Err(GatewayProtocolError::InvalidEvent),
            },
            NormalizedStreamKind::ToolIntents { intents } => {
                if !matches!(self.output, EventOutput::None) {
                    return Err(GatewayProtocolError::InvalidEvent);
                }
                self.output = EventOutput::Intents(intents.clone());
            }
            NormalizedStreamKind::Usage { .. } => {
                if self.usage_seen {
                    return Err(GatewayProtocolError::InvalidEvent);
                }
                self.usage_seen = true;
            }
            NormalizedStreamKind::Finished { completion } => {
                let matches = match (&self.output, completion) {
                    (EventOutput::Text(accumulated), NormalizedCompletion::Text { content }) => {
                        accumulated == content
                    }
                    (
                        EventOutput::Intents(proposed),
                        NormalizedCompletion::ToolIntents { intents },
                    ) => proposed == intents,
                    _ => false,
                };
                if !matches {
                    return Err(GatewayProtocolError::InvalidEvent);
                }
                self.terminal = true;
            }
            NormalizedStreamKind::Cancelled | NormalizedStreamKind::Failed { .. } => {
                self.terminal = true;
            }
        }
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(GatewayProtocolError::InvalidSequence)?;
        Ok(())
    }

    pub fn is_terminal(&self) -> bool {
        self.terminal
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GatewayPeerCredentials {
    pub pid: u32,
    pub uid: u32,
    pub gid: u32,
}

pub fn validate_gateway_peer(
    observed: GatewayPeerCredentials,
    expected_uid: u32,
    expected_gid: u32,
) -> Result<(), GatewayProtocolError> {
    if observed.pid == 0
        || expected_uid == 0
        || observed.uid != expected_uid
        || observed.gid != expected_gid
    {
        return Err(GatewayProtocolError::PeerCredentialMismatch);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
impl GatewayPeerCredentials {
    pub fn from_stream(
        stream: &std::os::unix::net::UnixStream,
    ) -> Result<Self, GatewayProtocolError> {
        use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};

        let credentials = getsockopt(stream, PeerCredentials)
            .map_err(|_| GatewayProtocolError::CredentialsUnavailable)?;
        Ok(Self {
            pid: u32::try_from(credentials.pid())
                .map_err(|_| GatewayProtocolError::CredentialsUnavailable)?,
            uid: credentials.uid(),
            gid: credentials.gid(),
        })
    }
}

#[cfg(all(unix, not(target_os = "linux")))]
impl GatewayPeerCredentials {
    pub fn from_stream(_: &std::os::unix::net::UnixStream) -> Result<Self, GatewayProtocolError> {
        Err(GatewayProtocolError::UnsupportedPlatform)
    }
}

fn encode_json(
    kind: GatewayMessageKind,
    value: &impl Serialize,
) -> Result<Vec<u8>, GatewayProtocolError> {
    let payload = serde_json::to_vec(value).map_err(|_| GatewayProtocolError::EncodingFailed)?;
    GatewayFrame::encode(kind, payload)
}

fn decode_json<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, GatewayProtocolError> {
    serde_json::from_slice(bytes).map_err(|_| GatewayProtocolError::MalformedPayload)
}

fn require_kind(
    frame: &GatewayFrame,
    expected: GatewayMessageKind,
) -> Result<(), GatewayProtocolError> {
    if frame.kind != expected {
        return Err(GatewayProtocolError::WrongMessageKind);
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), GatewayProtocolError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(GatewayProtocolError::IdentityMismatch);
    }
    Ok(())
}

fn require_canonical(original: &[u8], value: &impl Serialize) -> Result<(), GatewayProtocolError> {
    let canonical = serde_json::to_vec(value).map_err(|_| GatewayProtocolError::EncodingFailed)?;
    if canonical != original {
        return Err(GatewayProtocolError::NonCanonicalPayload);
    }
    Ok(())
}

fn validate_nonce(value: &str) -> Result<(), GatewayProtocolError> {
    if value.is_empty()
        || value.len() > MAX_INSTANCE_NONCE_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(GatewayProtocolError::IdentityMismatch);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatewayProtocolError {
    UnsupportedPlatform,
    CredentialsUnavailable,
    PeerCredentialMismatch,
    InvalidMagic,
    UnsupportedVersion,
    UnknownMessageKind,
    InvalidFlags,
    FrameTooLarge,
    DigestMismatch,
    TruncatedFrame,
    DecoderPoisoned,
    WrongMessageKind,
    MalformedPayload,
    NonCanonicalPayload,
    IdentityMismatch,
    InvalidRequest,
    InvalidEvent,
    InvalidSequence,
    EncodingFailed,
}

impl fmt::Display for GatewayProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedPlatform => "gateway credentials require Linux",
            Self::CredentialsUnavailable => "gateway peer credentials are unavailable",
            Self::PeerCredentialMismatch => "gateway peer credentials do not match",
            Self::InvalidMagic => "gateway frame magic is invalid",
            Self::UnsupportedVersion => "gateway protocol version is unsupported",
            Self::UnknownMessageKind => "gateway message kind is unknown",
            Self::InvalidFlags => "gateway frame flags are invalid",
            Self::FrameTooLarge => "gateway frame exceeds its bound",
            Self::DigestMismatch => "gateway frame digest does not match",
            Self::TruncatedFrame => "gateway frame is truncated",
            Self::DecoderPoisoned => "gateway decoder is permanently failed",
            Self::WrongMessageKind => "gateway message kind is not valid here",
            Self::MalformedPayload => "gateway payload is malformed",
            Self::NonCanonicalPayload => "gateway payload is not canonical",
            Self::IdentityMismatch => "gateway identity does not match",
            Self::InvalidRequest => "gateway inference request is invalid",
            Self::InvalidEvent => "gateway stream event is invalid",
            Self::InvalidSequence => "gateway stream sequence is invalid",
            Self::EncodingFailed => "gateway payload encoding failed",
        })
    }
}

impl std::error::Error for GatewayProtocolError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_runtime::{InferenceCancellation, ModelStreamState, ProviderStreamInput};

    fn request() -> InferenceRequest {
        InferenceRequest::synthetic(
            InferenceRequestId::parse("gateway-fixture-1".into()).unwrap(),
            ModelProviderKind::LlamaCpp,
            ModelProfile::parse("fixture-model:1".into()).unwrap(),
            vec![ConversationMessage::new(ConversationRole::User, "synthetic".into()).unwrap()],
            TurnIntentCatalogue::empty(),
            InferenceOutputMode::Text,
            2_000,
        )
        .unwrap()
    }

    fn one_frame(bytes: &[u8]) -> GatewayFrame {
        let mut decoder = GatewayFrameDecoder::default();
        let mut frames = Vec::new();
        for chunk in bytes.chunks(3) {
            frames.extend(decoder.push(chunk).unwrap());
        }
        decoder.finish().unwrap();
        assert_eq!(frames.len(), 1);
        frames.pop().unwrap()
    }

    #[test]
    fn framing_is_fragment_safe_and_digest_bound() {
        let encoded =
            encode_gateway_hello(GatewayProfile::LlamaCppCpuV1, &"a".repeat(64), "instance-1")
                .unwrap();
        assert_eq!(&encoded[..8], MAGIC);
        assert_eq!(encoded[10], GatewayMessageKind::Hello as u8);
        let frame = one_frame(&encoded);
        assert_eq!(
            decode_gateway_hello(&frame, GatewayProfile::LlamaCppCpuV1).unwrap(),
            ("a".repeat(64), "instance-1".into())
        );
        let mut tampered = encoded;
        *tampered.last_mut().unwrap() ^= 1;
        assert_eq!(
            GatewayFrameDecoder::default().push(&tampered),
            Err(GatewayProtocolError::DigestMismatch)
        );
    }

    #[test]
    fn framing_and_hello_expansion_poison_the_decoder_or_fail_closed() {
        let encoded =
            encode_gateway_hello(GatewayProfile::OllamaCpuV1, &"a".repeat(64), "instance-1")
                .unwrap();

        let mut unknown_kind = encoded.clone();
        unknown_kind[10] = 99;
        let mut decoder = GatewayFrameDecoder::default();
        assert_eq!(
            decoder.push(&unknown_kind),
            Err(GatewayProtocolError::UnknownMessageKind)
        );
        assert_eq!(
            decoder.push(&encoded),
            Err(GatewayProtocolError::DecoderPoisoned)
        );

        let mut oversized = encoded[..HEADER_BYTES].to_vec();
        oversized[12..16].copy_from_slice(
            &u32::try_from(MAX_GATEWAY_FRAME_BYTES + 1)
                .unwrap()
                .to_be_bytes(),
        );
        assert_eq!(
            GatewayFrameDecoder::default().push(&oversized),
            Err(GatewayProtocolError::FrameTooLarge)
        );
        assert_eq!(
            GatewayFrameDecoder::default()
                .push(&encoded[..HEADER_BYTES + 1])
                .and_then(|_| {
                    let mut decoder = GatewayFrameDecoder::default();
                    decoder.push(&encoded[..HEADER_BYTES + 1])?;
                    decoder.finish()
                }),
            Err(GatewayProtocolError::TruncatedFrame)
        );

        let expanded = GatewayFrame {
            kind: GatewayMessageKind::Hello,
            payload: format!(
                r#"{{"protocol_version":1,"model_protocol_version":1,"profile":"ollama_cpu_v1","boot_id_sha256":"{}","instance_nonce":"instance-1","authority":true}}"#,
                "a".repeat(64)
            )
            .into_bytes(),
        };
        assert_eq!(
            decode_gateway_hello(&expanded, GatewayProfile::OllamaCpuV1),
            Err(GatewayProtocolError::MalformedPayload)
        );

        let uppercase_digest = GatewayFrame {
            kind: GatewayMessageKind::Hello,
            payload: format!(
                r#"{{"protocol_version":1,"model_protocol_version":1,"profile":"ollama_cpu_v1","boot_id_sha256":"{}","instance_nonce":"instance-1"}}"#,
                "A".repeat(64)
            )
            .into_bytes(),
        };
        assert_eq!(
            decode_gateway_hello(&uppercase_digest, GatewayProfile::OllamaCpuV1),
            Err(GatewayProtocolError::IdentityMismatch)
        );
    }

    #[test]
    fn synthetic_request_is_canonical_and_private_expansion_fails() {
        let frame = one_frame(&encode_gateway_synthetic_request(&request()).unwrap());
        assert_eq!(
            decode_gateway_synthetic_request(&frame, GatewayProfile::LlamaCppCpuV1)
                .unwrap()
                .request_id()
                .as_str(),
            "gateway-fixture-1"
        );
        let offset = frame
            .payload()
            .windows(b"synthetic".len())
            .position(|part| part == b"synthetic")
            .unwrap();
        let mut payload = frame.payload().to_vec();
        payload.splice(
            offset..offset + b"synthetic".len(),
            b"private".iter().copied(),
        );
        assert_eq!(
            decode_gateway_synthetic_request(
                &GatewayFrame {
                    kind: GatewayMessageKind::SyntheticInference,
                    payload,
                },
                GatewayProfile::LlamaCppCpuV1,
            ),
            Err(GatewayProtocolError::MalformedPayload)
        );
        assert_eq!(
            decode_gateway_synthetic_request(&frame, GatewayProfile::OllamaCpuV1),
            Err(GatewayProtocolError::IdentityMismatch)
        );
    }

    #[test]
    fn private_frame_contains_no_provider_authority_and_gateway_injects_it() {
        let payload = br#"{"version":1,"request_id":"private-1","messages":[{"role":"user","content":"private content"}],"intents":{"eligible":[]},"output_mode":"text","deadline_ms":2000}"#.to_vec();
        assert!(
            !payload
                .windows(b"provider".len())
                .any(|part| part == b"provider")
        );
        assert!(!payload.windows(b"model".len()).any(|part| part == b"model"));
        assert!(
            !payload
                .windows(b"classification".len())
                .any(|part| part == b"classification")
        );
        let frame = one_frame(
            &GatewayFrame::encode(GatewayMessageKind::PrivateInference, payload).unwrap(),
        );
        let request = decode_gateway_private_request(
            &frame,
            ModelProviderKind::LlamaCpp,
            ModelProfile::parse("qwen2.5:0.5b".into()).unwrap(),
        )
        .unwrap();
        assert_eq!(request.provider(), ModelProviderKind::LlamaCpp);
        assert_eq!(request.model().as_str(), "qwen2.5:0.5b");
        let encoded = serde_json::to_string(&request).unwrap();
        assert!(encoded.contains(r#""input_classification":"private""#));

        let expanded = GatewayFrame {
            kind: GatewayMessageKind::PrivateInference,
            payload: br#"{"version":1,"request_id":"private-1","provider":"llama_cpp","messages":[{"role":"user","content":"private content"}],"intents":{"eligible":[]},"output_mode":"text","deadline_ms":2000}"#.to_vec(),
        };
        assert_eq!(
            decode_gateway_private_request(
                &expanded,
                ModelProviderKind::LlamaCpp,
                ModelProfile::parse("qwen2.5:0.5b".into()).unwrap(),
            ),
            Err(GatewayProtocolError::MalformedPayload)
        );
        assert_eq!(
            decode_gateway_synthetic_request(&frame, GatewayProfile::LlamaCppCpuV1),
            Err(GatewayProtocolError::WrongMessageKind)
        );
    }

    #[test]
    fn normalized_events_round_trip_and_sequence_fails_closed() {
        let request = request();
        let mut source = ModelStreamState::new(&request, InferenceCancellation::new());
        let events = [
            source.apply(0, ProviderStreamInput::Started).unwrap(),
            source
                .apply(1, ProviderStreamInput::TextDelta("hello".into()))
                .unwrap(),
            source.apply(2, ProviderStreamInput::Finished).unwrap(),
        ];
        let mut validator = GatewayEventValidator::new(request.request_id());
        for event in &events {
            let decoded =
                decode_gateway_event(&one_frame(&encode_gateway_event(event).unwrap())).unwrap();
            validator.accept(&decoded).unwrap();
        }
        assert!(validator.is_terminal());
        assert_eq!(
            validator.accept(&events[2]),
            Err(GatewayProtocolError::InvalidSequence)
        );

        let mut changed_completion = events[2].clone();
        changed_completion.event = NormalizedStreamKind::Finished {
            completion: NormalizedCompletion::Text {
                content: "different".into(),
            },
        };
        let mut validator = GatewayEventValidator::new(request.request_id());
        validator.accept(&events[0]).unwrap();
        validator.accept(&events[1]).unwrap();
        assert_eq!(
            validator.accept(&changed_completion),
            Err(GatewayProtocolError::InvalidEvent)
        );

        let cancellation = InferenceCancellation::new();
        cancellation.cancel();
        let mut cancelled_source = ModelStreamState::new(&request, cancellation);
        let cancelled = cancelled_source
            .apply(0, ProviderStreamInput::Started)
            .unwrap();
        let mut validator = GatewayEventValidator::new(request.request_id());
        validator.accept(&cancelled).unwrap();
        assert!(validator.is_terminal());
    }

    #[test]
    fn cancel_is_closed_canonical_and_request_bound() {
        let request_id = InferenceRequestId::parse("gateway-fixture-1".into()).unwrap();
        let frame = one_frame(&encode_gateway_cancel(&request_id).unwrap());
        assert_eq!(
            decode_gateway_cancel(&frame).unwrap().as_str(),
            request_id.as_str()
        );
        let mut noncanonical = frame.payload().to_vec();
        noncanonical.push(b' ');
        assert_eq!(
            decode_gateway_cancel(&GatewayFrame {
                kind: GatewayMessageKind::Cancel,
                payload: noncanonical,
            }),
            Err(GatewayProtocolError::NonCanonicalPayload)
        );
    }

    #[test]
    fn credentials_reject_root_wrong_identity_and_zero_pid() {
        let valid = GatewayPeerCredentials {
            pid: 42,
            uid: 981,
            gid: 981,
        };
        assert_eq!(validate_gateway_peer(valid, 981, 981), Ok(()));
        assert_eq!(
            validate_gateway_peer(valid, 982, 981),
            Err(GatewayProtocolError::PeerCredentialMismatch)
        );
        assert_eq!(
            validate_gateway_peer(valid, 0, 981),
            Err(GatewayProtocolError::PeerCredentialMismatch)
        );
        assert_eq!(
            validate_gateway_peer(GatewayPeerCredentials { pid: 0, ..valid }, 981, 981),
            Err(GatewayProtocolError::PeerCredentialMismatch)
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn target_linux_reads_credentials_from_connected_descriptor() {
        let (left, right) = std::os::unix::net::UnixStream::pair().unwrap();
        let left_peer = GatewayPeerCredentials::from_stream(&left).unwrap();
        let right_peer = GatewayPeerCredentials::from_stream(&right).unwrap();
        assert_eq!(left_peer.uid, nix::unistd::geteuid().as_raw());
        assert_eq!(right_peer.uid, nix::unistd::geteuid().as_raw());
        assert_ne!(left_peer.pid, 0);
        assert_ne!(right_peer.pid, 0);
    }
}
