//! Provider-neutral, authority-free model inference contract.
//!
//! This module contains no provider transport and no path to the capability
//! broker or executor. Provider output is untrusted data until it passes these
//! closed validators, and even a validated intent remains only a proposal.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

mod gateway;
#[cfg(unix)]
mod gateway_fixture;
mod llama_cpp;
mod ollama;
mod provider_profile;
mod runtime_readiness;

pub use gateway::{
    GATEWAY_PROTOCOL_VERSION, GatewayEventValidator, GatewayFrame, GatewayFrameDecoder,
    GatewayMessageKind, GatewayPeerCredentials, GatewayProfile, GatewayProtocolError,
    MAX_GATEWAY_FRAME_BYTES, decode_gateway_cancel, decode_gateway_event, decode_gateway_hello,
    decode_gateway_synthetic_request, encode_gateway_cancel, encode_gateway_event,
    encode_gateway_hello, encode_gateway_synthetic_request, validate_gateway_peer,
};
#[cfg(unix)]
pub use gateway_fixture::{
    GatewayFixtureError, SyntheticGatewayClient, fixed_synthetic_gateway_request,
    serve_synthetic_gateway_once,
};
pub use llama_cpp::{LLAMA_CPP_ENDPOINT, LlamaCppAdapter, LlamaCppAdapterError};
pub use ollama::{OLLAMA_ENDPOINT, OllamaAdapter, OllamaAdapterError};
pub use provider_profile::{
    MAX_PROVIDER_MANIFEST_BYTES, ProviderArtifact, ProviderFilesystemPolicy, ProviderProfileError,
    ProviderProfileManifest, ProviderProfileResources, ProviderProfileSpec,
    ProviderServiceIdentity, ValidatedProviderProfile, load_installed_provider_profile,
};
#[cfg(debug_assertions)]
pub use provider_profile::{SyntheticProviderPackage, fixed_synthetic_provider_package};
pub use runtime_readiness::{
    AccountDatabaseEvidence, ResolvedModelIdentities, RuntimeFileEvidence, RuntimeReadinessError,
    RuntimeReadinessEvidence, load_installed_runtime_readiness,
};

pub const MODEL_PROTOCOL_VERSION: u16 = 1;
pub const MAX_INFERENCE_REQUEST_BYTES: usize = 256 * 1024;
pub const MAX_MESSAGES: usize = 64;
pub const MAX_MESSAGE_BYTES: usize = 32 * 1024;
pub const MAX_TOTAL_MESSAGE_BYTES: usize = 192 * 1024;
pub const MAX_MODEL_PROFILE_BYTES: usize = 128;
pub const MAX_TOOL_INTENTS: usize = 8;
pub const MAX_TOOL_ARGUMENT_BYTES: usize = 16 * 1024;
pub const MAX_TEXT_DELTA_BYTES: usize = 8 * 1024;
pub const MAX_OUTPUT_BYTES: usize = 128 * 1024;
pub const MAX_DEADLINE_MS: u64 = 120_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelProviderKind {
    Ollama,
    LlamaCpp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ModelProfile(String);

impl ModelProfile {
    pub fn parse(value: String) -> Result<Self, ModelContractError> {
        let valid = !value.is_empty()
            && value.len() <= MAX_MODEL_PROFILE_BYTES
            && !value.contains("://")
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b':' | b'/' | b'_' | b'-')
            });
        valid
            .then_some(Self(value))
            .ok_or(ModelContractError::InvalidModelProfile)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InferenceRequestId(String);

impl InferenceRequestId {
    pub fn parse(value: String) -> Result<Self, ModelContractError> {
        let valid = !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
        valid
            .then_some(Self(value))
            .ok_or(ModelContractError::InvalidRequestId)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationMessage {
    role: ConversationRole,
    content: String,
}

impl ConversationMessage {
    pub fn new(role: ConversationRole, content: String) -> Result<Self, ModelContractError> {
        if content.is_empty() || content.len() > MAX_MESSAGE_BYTES || content.contains('\0') {
            return Err(ModelContractError::InvalidMessage);
        }
        Ok(Self { role, content })
    }

    pub fn role(&self) -> ConversationRole {
        self.role
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceOutputMode {
    Text,
    BlossomTurn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelIntentKind {
    SystemOsIdentity,
    SystemUptime,
    SystemMemorySummary,
    SystemStorageSummary,
    ProcessSelf,
    ProcessList,
}

impl ModelIntentKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::SystemOsIdentity => "system.os.identity",
            Self::SystemUptime => "system.uptime",
            Self::SystemMemorySummary => "system.memory.summary",
            Self::SystemStorageSummary => "system.storage.summary",
            Self::ProcessSelf => "process.self",
            Self::ProcessList => "process.list",
        }
    }

    fn from_name(value: &str) -> Option<Self> {
        match value {
            "system.os.identity" => Some(Self::SystemOsIdentity),
            "system.uptime" => Some(Self::SystemUptime),
            "system.memory.summary" => Some(Self::SystemMemorySummary),
            "system.storage.summary" => Some(Self::SystemStorageSummary),
            "process.self" => Some(Self::ProcessSelf),
            "process.list" => Some(Self::ProcessList),
            _ => None,
        }
    }

    pub fn definition(self) -> ModelIntentDefinition {
        ModelIntentDefinition {
            name: self.name(),
            description: match self {
                Self::SystemOsIdentity => "Read the operating-system identity.",
                Self::SystemUptime => "Read the system uptime.",
                Self::SystemMemorySummary => "Read a bounded memory summary.",
                Self::SystemStorageSummary => "Read a bounded root-storage summary.",
                Self::ProcessSelf => "Read the current Blossom process identity.",
                Self::ProcessList => "Propose reading a bounded process list.",
            },
            parameters: EmptyIntentParameters::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EmptyIntentParameters {
    #[serde(rename = "type")]
    parameter_type: EmptyObjectType,
    properties: EmptyProperties,
    #[serde(rename = "additionalProperties")]
    additional_properties: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
enum EmptyObjectType {
    #[default]
    #[serde(rename = "object")]
    Object,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
struct EmptyProperties {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ModelIntentDefinition {
    name: &'static str,
    description: &'static str,
    parameters: EmptyIntentParameters,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TurnIntentCatalogue {
    eligible: BTreeSet<ModelIntentKind>,
}

impl TurnIntentCatalogue {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.eligible.is_empty()
    }

    pub fn contains(&self, intent: ModelIntentKind) -> bool {
        self.eligible.contains(&intent)
    }

    pub fn iter(&self) -> impl Iterator<Item = ModelIntentKind> + '_ {
        self.eligible.iter().copied()
    }

    #[allow(
        dead_code,
        reason = "the first provider adapter will construct per-turn catalogues"
    )]
    pub(crate) fn from_eligible(
        eligible: impl IntoIterator<Item = ModelIntentKind>,
    ) -> Result<Self, ModelContractError> {
        let eligible: BTreeSet<_> = eligible.into_iter().collect();
        if eligible.len() > MAX_TOOL_INTENTS {
            return Err(ModelContractError::TooManyIntents);
        }
        Ok(Self { eligible })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum InputClassification {
    #[allow(
        dead_code,
        reason = "construction stays internal until a provider adapter exists"
    )]
    Synthetic,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InferenceRequest {
    version: u16,
    request_id: InferenceRequestId,
    provider: ModelProviderKind,
    model: ModelProfile,
    input_classification: InputClassification,
    messages: Vec<ConversationMessage>,
    intents: TurnIntentCatalogue,
    output_mode: InferenceOutputMode,
    deadline_ms: u64,
}

impl InferenceRequest {
    #[allow(
        dead_code,
        reason = "construction stays internal until a provider adapter exists"
    )]
    pub(crate) fn synthetic(
        request_id: InferenceRequestId,
        provider: ModelProviderKind,
        model: ModelProfile,
        messages: Vec<ConversationMessage>,
        intents: TurnIntentCatalogue,
        output_mode: InferenceOutputMode,
        deadline_ms: u64,
    ) -> Result<Self, ModelContractError> {
        let request = Self {
            version: MODEL_PROTOCOL_VERSION,
            request_id,
            provider,
            model,
            input_classification: InputClassification::Synthetic,
            messages,
            intents,
            output_mode,
            deadline_ms,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> Result<(), ModelContractError> {
        if self.version != MODEL_PROTOCOL_VERSION {
            return Err(ModelContractError::UnsupportedVersion);
        }
        if self.messages.is_empty() || self.messages.len() > MAX_MESSAGES {
            return Err(ModelContractError::InvalidMessageCount);
        }
        let total = self.messages.iter().try_fold(0usize, |total, message| {
            if message.content.is_empty()
                || message.content.len() > MAX_MESSAGE_BYTES
                || message.content.contains('\0')
            {
                return Err(ModelContractError::InvalidMessage);
            }
            total
                .checked_add(message.content.len())
                .ok_or(ModelContractError::RequestTooLarge)
        })?;
        if total > MAX_TOTAL_MESSAGE_BYTES {
            return Err(ModelContractError::RequestTooLarge);
        }
        if self.deadline_ms == 0 || self.deadline_ms > MAX_DEADLINE_MS {
            return Err(ModelContractError::InvalidDeadline);
        }
        let encoded = serde_json::to_vec(self).map_err(|_| ModelContractError::EncodingFailed)?;
        if encoded.len() > MAX_INFERENCE_REQUEST_BYTES {
            return Err(ModelContractError::RequestTooLarge);
        }
        Ok(())
    }

    pub fn request_id(&self) -> &InferenceRequestId {
        &self.request_id
    }

    pub fn provider(&self) -> ModelProviderKind {
        self.provider.clone()
    }

    pub fn model(&self) -> &ModelProfile {
        &self.model
    }

    pub fn messages(&self) -> &[ConversationMessage] {
        &self.messages
    }

    pub fn intents(&self) -> &TurnIntentCatalogue {
        &self.intents
    }

    pub fn output_mode(&self) -> InferenceOutputMode {
        self.output_mode
    }

    pub fn deadline_ms(&self) -> u64 {
        self.deadline_ms
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ProviderCompletion {
    Text { content: String },
    ToolIntents { intents: Vec<ProviderToolIntent> },
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderToolIntent {
    name: String,
    arguments: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedToolIntent {
    kind: ModelIntentKind,
}

impl ProposedToolIntent {
    pub fn kind(&self) -> ModelIntentKind {
        self.kind
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NormalizedCompletion {
    Text { content: String },
    ToolIntents { intents: Vec<ProposedToolIntent> },
}

pub fn validate_provider_completion(
    bytes: &[u8],
    catalogue: &TurnIntentCatalogue,
) -> Result<NormalizedCompletion, ModelContractError> {
    if bytes.is_empty() || bytes.len() > MAX_OUTPUT_BYTES {
        return Err(ModelContractError::OutputTooLarge);
    }
    let completion: ProviderCompletion =
        serde_json::from_slice(bytes).map_err(|_| ModelContractError::MalformedOutput)?;
    match completion {
        ProviderCompletion::Text { content } => {
            if content.is_empty() || content.len() > MAX_OUTPUT_BYTES || content.contains('\0') {
                return Err(ModelContractError::InvalidText);
            }
            Ok(NormalizedCompletion::Text { content })
        }
        ProviderCompletion::ToolIntents { intents } => {
            if intents.is_empty() || intents.len() > MAX_TOOL_INTENTS {
                return Err(ModelContractError::TooManyIntents);
            }
            let mut normalized = Vec::with_capacity(intents.len());
            let mut seen = BTreeSet::new();
            for intent in intents {
                let kind = ModelIntentKind::from_name(&intent.name)
                    .ok_or(ModelContractError::UnknownIntent)?;
                if !catalogue.contains(kind) {
                    return Err(ModelContractError::IntentNotEligible);
                }
                if !seen.insert(kind) {
                    return Err(ModelContractError::DuplicateIntent);
                }
                let argument_bytes = serde_json::to_vec(&intent.arguments)
                    .map_err(|_| ModelContractError::MalformedOutput)?;
                if argument_bytes.len() > MAX_TOOL_ARGUMENT_BYTES
                    || intent.arguments != Value::Object(Default::default())
                {
                    return Err(ModelContractError::InvalidIntentArguments);
                }
                normalized.push(ProposedToolIntent { kind });
            }
            Ok(NormalizedCompletion::ToolIntents {
                intents: normalized,
            })
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct InferenceCancellation {
    cancelled: Arc<AtomicBool>,
}

impl InferenceCancellation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderFailureCategory {
    Unavailable,
    TimedOut,
    Disconnected,
    Malformed,
    ProviderFailed,
    OutputLimit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderStreamInput {
    Started,
    TextDelta(String),
    ToolIntents(Vec<u8>),
    Usage {
        prompt_tokens: u64,
        generated_tokens: u64,
    },
    Finished,
    Failed(ProviderFailureCategory),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NormalizedStreamKind {
    Started,
    TextDelta {
        content: String,
    },
    ToolIntents {
        intents: Vec<ProposedToolIntent>,
    },
    Usage {
        prompt_tokens: u64,
        generated_tokens: u64,
    },
    Finished {
        completion: NormalizedCompletion,
    },
    Cancelled,
    Failed {
        category: ProviderFailureCategory,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedStreamEvent {
    pub version: u16,
    pub request_id: String,
    pub sequence: u64,
    pub event: NormalizedStreamKind,
}

#[derive(Debug)]
pub struct ModelStreamState {
    request_id: InferenceRequestId,
    catalogue: TurnIntentCatalogue,
    cancellation: InferenceCancellation,
    next_sequence: u64,
    started: bool,
    terminal: bool,
    text: String,
    intents: Option<Vec<ProposedToolIntent>>,
    usage_seen: bool,
}

impl ModelStreamState {
    pub fn new(request: &InferenceRequest, cancellation: InferenceCancellation) -> Self {
        Self {
            request_id: request.request_id.clone(),
            catalogue: request.intents.clone(),
            cancellation,
            next_sequence: 0,
            started: false,
            terminal: false,
            text: String::new(),
            intents: None,
            usage_seen: false,
        }
    }

    pub fn apply(
        &mut self,
        sequence: u64,
        input: ProviderStreamInput,
    ) -> Result<NormalizedStreamEvent, ModelContractError> {
        let result = self.apply_transition(sequence, input);
        if result.is_err() {
            self.terminal = true;
        }
        result
    }

    fn apply_transition(
        &mut self,
        sequence: u64,
        input: ProviderStreamInput,
    ) -> Result<NormalizedStreamEvent, ModelContractError> {
        if self.terminal {
            return Err(ModelContractError::EventAfterTerminal);
        }
        if sequence != self.next_sequence {
            return Err(ModelContractError::InvalidSequence);
        }
        if self.cancellation.is_cancelled() {
            self.terminal = true;
            return Ok(self.event(sequence, NormalizedStreamKind::Cancelled));
        }
        let event = match input {
            ProviderStreamInput::Started if !self.started && sequence == 0 => {
                self.started = true;
                NormalizedStreamKind::Started
            }
            ProviderStreamInput::Started => return Err(ModelContractError::InvalidStreamState),
            _ if !self.started => return Err(ModelContractError::InvalidStreamState),
            ProviderStreamInput::TextDelta(content) => {
                if self.intents.is_some()
                    || content.is_empty()
                    || content.len() > MAX_TEXT_DELTA_BYTES
                    || content.contains('\0')
                    || self.text.len().saturating_add(content.len()) > MAX_OUTPUT_BYTES
                {
                    return Err(ModelContractError::InvalidText);
                }
                self.text.push_str(&content);
                NormalizedStreamKind::TextDelta { content }
            }
            ProviderStreamInput::ToolIntents(bytes) => {
                if !self.text.is_empty() || self.intents.is_some() {
                    return Err(ModelContractError::MixedCompletion);
                }
                let NormalizedCompletion::ToolIntents { intents } =
                    validate_provider_completion(&bytes, &self.catalogue)?
                else {
                    return Err(ModelContractError::MixedCompletion);
                };
                self.intents = Some(intents.clone());
                NormalizedStreamKind::ToolIntents { intents }
            }
            ProviderStreamInput::Usage {
                prompt_tokens,
                generated_tokens,
            } => {
                if self.usage_seen {
                    return Err(ModelContractError::InvalidStreamState);
                }
                self.usage_seen = true;
                NormalizedStreamKind::Usage {
                    prompt_tokens,
                    generated_tokens,
                }
            }
            ProviderStreamInput::Finished => {
                let completion = if let Some(intents) = self.intents.clone() {
                    NormalizedCompletion::ToolIntents { intents }
                } else if !self.text.is_empty() {
                    NormalizedCompletion::Text {
                        content: self.text.clone(),
                    }
                } else {
                    return Err(ModelContractError::InvalidStreamState);
                };
                self.terminal = true;
                NormalizedStreamKind::Finished { completion }
            }
            ProviderStreamInput::Failed(category) => {
                self.terminal = true;
                NormalizedStreamKind::Failed { category }
            }
        };
        let result = self.event(sequence, event);
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(ModelContractError::InvalidSequence)?;
        Ok(result)
    }

    fn event(&self, sequence: u64, event: NormalizedStreamKind) -> NormalizedStreamEvent {
        NormalizedStreamEvent {
            version: MODEL_PROTOCOL_VERSION,
            request_id: self.request_id.as_str().into(),
            sequence,
            event,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceAuditOutcome {
    Started,
    Text,
    ProposedIntents,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InferenceAuditProjection {
    pub request_id: String,
    pub provider: ModelProviderKind,
    pub model_sha256: String,
    pub request_sha256: String,
    pub outcome: InferenceAuditOutcome,
    pub output_bytes: usize,
    pub proposed_intents: usize,
    pub prompt_tokens: Option<u64>,
    pub generated_tokens: Option<u64>,
}

impl InferenceAuditProjection {
    pub fn started(request: &InferenceRequest) -> Result<Self, ModelContractError> {
        request.validate()?;
        let request_bytes =
            serde_json::to_vec(request).map_err(|_| ModelContractError::EncodingFailed)?;
        Ok(Self {
            request_id: request.request_id.as_str().into(),
            provider: request.provider.clone(),
            model_sha256: digest(request.model.as_str().as_bytes()),
            request_sha256: digest(&request_bytes),
            outcome: InferenceAuditOutcome::Started,
            output_bytes: 0,
            proposed_intents: 0,
            prompt_tokens: None,
            generated_tokens: None,
        })
    }

    pub fn finished(
        request: &InferenceRequest,
        completion: Option<&NormalizedCompletion>,
        cancelled: bool,
        failed: bool,
        usage: Option<(u64, u64)>,
    ) -> Result<Self, ModelContractError> {
        let mut projection = Self::started(request)?;
        let (outcome, output_bytes, proposed_intents) = if cancelled {
            (InferenceAuditOutcome::Cancelled, 0, 0)
        } else if failed {
            (InferenceAuditOutcome::Failed, 0, 0)
        } else {
            match completion.ok_or(ModelContractError::InvalidStreamState)? {
                NormalizedCompletion::Text { content } => {
                    (InferenceAuditOutcome::Text, content.len(), 0)
                }
                NormalizedCompletion::ToolIntents { intents } => {
                    (InferenceAuditOutcome::ProposedIntents, 0, intents.len())
                }
            }
        };
        projection.outcome = outcome;
        projection.output_bytes = output_bytes;
        projection.proposed_intents = proposed_intents;
        if let Some((prompt, generated)) = usage {
            projection.prompt_tokens = Some(prompt);
            projection.generated_tokens = Some(generated);
        }
        Ok(projection)
    }
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelContractError {
    UnsupportedVersion,
    InvalidRequestId,
    InvalidModelProfile,
    InvalidMessage,
    InvalidMessageCount,
    InvalidDeadline,
    RequestTooLarge,
    OutputTooLarge,
    InvalidText,
    MalformedOutput,
    UnknownIntent,
    IntentNotEligible,
    InvalidIntentArguments,
    DuplicateIntent,
    TooManyIntents,
    MixedCompletion,
    InvalidSequence,
    InvalidStreamState,
    EventAfterTerminal,
    EncodingFailed,
}

impl fmt::Display for ModelContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedVersion => "unsupported model protocol version",
            Self::InvalidRequestId => "invalid inference request identifier",
            Self::InvalidModelProfile => "invalid model profile",
            Self::InvalidMessage => "invalid conversation message",
            Self::InvalidMessageCount => "invalid conversation message count",
            Self::InvalidDeadline => "invalid inference deadline",
            Self::RequestTooLarge => "inference request exceeds its bound",
            Self::OutputTooLarge => "provider output exceeds its bound",
            Self::InvalidText => "provider text is invalid",
            Self::MalformedOutput => "provider output is malformed",
            Self::UnknownIntent => "provider proposed an unknown intent",
            Self::IntentNotEligible => "provider intent is not eligible for this turn",
            Self::InvalidIntentArguments => "provider intent arguments are invalid",
            Self::DuplicateIntent => "provider proposed a duplicate intent",
            Self::TooManyIntents => "provider proposed too many intents",
            Self::MixedCompletion => "provider mixed text and tool intents",
            Self::InvalidSequence => "provider stream sequence is invalid",
            Self::InvalidStreamState => "provider stream transition is invalid",
            Self::EventAfterTerminal => "provider event followed a terminal event",
            Self::EncodingFailed => "model contract encoding failed",
        })
    }
}

impl std::error::Error for ModelContractError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(intents: TurnIntentCatalogue) -> InferenceRequest {
        InferenceRequest::synthetic(
            InferenceRequestId::parse("model-1".into()).unwrap(),
            ModelProviderKind::Ollama,
            ModelProfile::parse("fixture-model:1".into()).unwrap(),
            vec![
                ConversationMessage::new(ConversationRole::User, "synthetic hello".into()).unwrap(),
            ],
            intents,
            InferenceOutputMode::BlossomTurn,
            5_000,
        )
        .unwrap()
    }

    #[test]
    fn request_fixture_is_closed_bounded_and_byte_stable() {
        let request = request(TurnIntentCatalogue::empty());
        assert_eq!(request.validate(), Ok(()));
        assert_eq!(
            serde_json::to_string(&request).unwrap(),
            r#"{"version":1,"request_id":"model-1","provider":"ollama","model":"fixture-model:1","input_classification":"synthetic","messages":[{"role":"user","content":"synthetic hello"}],"intents":{"eligible":[]},"output_mode":"blossom_turn","deadline_ms":5000}"#
        );
        assert!(
            ConversationMessage::new(ConversationRole::User, "x".repeat(MAX_MESSAGE_BYTES + 1))
                .is_err()
        );
        assert!(ModelProfile::parse("https://remote.invalid/model".into()).is_err());
    }

    #[test]
    fn catalogue_is_empty_by_default_and_rejects_unlisted_intents() {
        let empty = TurnIntentCatalogue::empty();
        assert!(empty.is_empty());
        assert_eq!(
            validate_provider_completion(
                br#"{"kind":"tool_intents","intents":[{"name":"process.list","arguments":{}}]}"#,
                &empty,
            ),
            Err(ModelContractError::IntentNotEligible)
        );
        let eligible = TurnIntentCatalogue::from_eligible([ModelIntentKind::ProcessList]).unwrap();
        assert!(matches!(
            validate_provider_completion(
                br#"{"kind":"tool_intents","intents":[{"name":"process.list","arguments":{}}]}"#,
                &eligible,
            ),
            Ok(NormalizedCompletion::ToolIntents { .. })
        ));
    }

    #[test]
    fn completion_schema_rejects_mixed_unknown_and_argument_expansion() {
        let eligible = TurnIntentCatalogue::from_eligible([ModelIntentKind::ProcessList]).unwrap();
        assert_eq!(
            validate_provider_completion(
                br#"{"kind":"text","content":"hello","intents":[]}"#,
                &eligible,
            ),
            Err(ModelContractError::MalformedOutput)
        );
        assert_eq!(
            validate_provider_completion(
                br#"{"kind":"tool_intents","intents":[{"name":"shell.execute","arguments":{}}]}"#,
                &eligible,
            ),
            Err(ModelContractError::UnknownIntent)
        );
        assert_eq!(
            validate_provider_completion(
                br#"{"kind":"tool_intents","intents":[{"name":"process.list","arguments":{"path":"~/.ssh"}}]}"#,
                &eligible,
            ),
            Err(ModelContractError::InvalidIntentArguments)
        );
        assert_eq!(
            validate_provider_completion(
                br#"{"kind":"tool_intents","intents":[{"name":"process.list","arguments":{}},{"name":"process.list","arguments":{}}]}"#,
                &eligible,
            ),
            Err(ModelContractError::DuplicateIntent)
        );
    }

    #[test]
    fn stream_is_ordered_single_terminal_and_rejects_mixed_output() {
        let request =
            request(TurnIntentCatalogue::from_eligible([ModelIntentKind::ProcessList]).unwrap());
        let mut stream = ModelStreamState::new(&request, InferenceCancellation::new());
        assert!(matches!(
            stream.apply(0, ProviderStreamInput::Started).unwrap().event,
            NormalizedStreamKind::Started
        ));
        stream
            .apply(1, ProviderStreamInput::TextDelta("hello".into()))
            .unwrap();
        assert_eq!(
            stream.apply(
                2,
                ProviderStreamInput::ToolIntents(
                    br#"{"kind":"tool_intents","intents":[{"name":"process.list","arguments":{}}]}"#.to_vec(),
                ),
            ),
            Err(ModelContractError::MixedCompletion)
        );
        assert_eq!(
            stream.apply(2, ProviderStreamInput::Finished),
            Err(ModelContractError::EventAfterTerminal)
        );
    }

    #[test]
    fn cancellation_wins_and_releases_no_completed_intent() {
        let cancellation = InferenceCancellation::new();
        let request =
            request(TurnIntentCatalogue::from_eligible([ModelIntentKind::ProcessList]).unwrap());
        let mut stream = ModelStreamState::new(&request, cancellation.clone());
        stream.apply(0, ProviderStreamInput::Started).unwrap();
        cancellation.cancel();
        let event = stream
            .apply(
                1,
                ProviderStreamInput::ToolIntents(
                    br#"{"kind":"tool_intents","intents":[{"name":"process.list","arguments":{}}]}"#.to_vec(),
                ),
            )
            .unwrap();
        assert!(matches!(event.event, NormalizedStreamKind::Cancelled));
        assert_eq!(
            stream.apply(2, ProviderStreamInput::Finished),
            Err(ModelContractError::EventAfterTerminal)
        );
    }

    #[test]
    fn protocol_error_permanently_poisons_stream() {
        let request = request(TurnIntentCatalogue::empty());
        let mut stream = ModelStreamState::new(&request, InferenceCancellation::new());
        assert_eq!(
            stream.apply(1, ProviderStreamInput::Started),
            Err(ModelContractError::InvalidSequence)
        );
        assert_eq!(
            stream.apply(0, ProviderStreamInput::Started),
            Err(ModelContractError::EventAfterTerminal)
        );
    }

    #[test]
    fn audit_projection_contains_digests_not_private_content() {
        let request = request(TurnIntentCatalogue::empty());
        let completion = NormalizedCompletion::Text {
            content: "generated secret".into(),
        };
        let audit = InferenceAuditProjection::finished(
            &request,
            Some(&completion),
            false,
            false,
            Some((4, 2)),
        )
        .unwrap();
        let json = serde_json::to_string(&audit).unwrap();
        assert!(!json.contains("synthetic hello"));
        assert!(!json.contains("generated secret"));
        assert!(!json.contains("fixture-model"));
        assert_eq!(audit.output_bytes, "generated secret".len());
        assert_eq!(audit.prompt_tokens, Some(4));
    }

    #[test]
    fn normalized_event_fixture_is_byte_stable() {
        let request = request(TurnIntentCatalogue::empty());
        let mut stream = ModelStreamState::new(&request, InferenceCancellation::new());
        let event = stream.apply(0, ProviderStreamInput::Started).unwrap();
        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"version":1,"request_id":"model-1","sequence":0,"event":{"kind":"started"}}"#
        );
    }

    #[test]
    fn tool_definition_is_code_owned_empty_and_byte_stable() {
        assert_eq!(
            serde_json::to_string(&ModelIntentKind::ProcessList.definition()).unwrap(),
            r#"{"name":"process.list","description":"Propose reading a bounded process list.","parameters":{"type":"object","properties":{},"additionalProperties":false}}"#
        );
    }
}
