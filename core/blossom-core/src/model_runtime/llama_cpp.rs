//! Fixed-loopback llama.cpp development adapter.
//!
//! The adapter accepts only crate-internal synthetic requests and only the
//! OpenAI-compatible chat-completions SSE surface. It never calls llama.cpp's
//! model, media, server-tool, agent, control, or administrative endpoints.

use super::ollama::{
    OllamaAdapterError, configure_stream, connect_checked, read_body, read_headers, write_request,
};
use super::{
    ConversationRole, InferenceCancellation, InferenceOutputMode, InferenceRequest,
    ModelContractError, ModelIntentDefinition, ModelProviderKind, ModelStreamState,
    NormalizedStreamEvent, ProviderFailureCategory, ProviderStreamInput,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
use std::io::BufReader;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::{Duration, Instant};

pub const LLAMA_CPP_ENDPOINT: &str = "127.0.0.1:8080";
const LLAMA_CPP_PATH: &str = "/v1/chat/completions";
const MAX_GENERATED_TOKENS: u32 = 512;
const MAX_SSE_EVENT_BYTES: usize = 192 * 1024;
const MAX_STREAM_EVENTS: u64 = 4_096;
const MAX_STREAM_ID_BYTES: usize = 128;
const MAX_TOOL_CALL_ID_BYTES: usize = 128;
const MAX_TOOL_NAME_BYTES: usize = 128;
const MAX_TOOL_ARGUMENT_BYTES: usize = 16 * 1024;
const MAX_METADATA_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug)]
pub struct LlamaCppAdapter {
    endpoint: SocketAddr,
}

impl Default for LlamaCppAdapter {
    fn default() -> Self {
        Self {
            endpoint: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8_080)),
        }
    }
}

impl LlamaCppAdapter {
    #[cfg(test)]
    fn for_test(endpoint: SocketAddr) -> Result<Self, LlamaCppAdapterError> {
        if !endpoint.ip().is_loopback() {
            return Err(LlamaCppAdapterError::InvalidEndpoint);
        }
        Ok(Self { endpoint })
    }

    pub fn stream(
        &self,
        request: &InferenceRequest,
        cancellation: InferenceCancellation,
        mut emit: impl FnMut(&NormalizedStreamEvent),
    ) -> Result<(), LlamaCppAdapterError> {
        self.infer_inner(request, cancellation, &mut emit)
            .map(|_| ())
    }

    #[cfg(test)]
    fn infer(
        &self,
        request: &InferenceRequest,
        cancellation: InferenceCancellation,
    ) -> Result<Vec<NormalizedStreamEvent>, LlamaCppAdapterError> {
        self.infer_inner(request, cancellation, &mut |_| {})
    }

    fn infer_inner(
        &self,
        request: &InferenceRequest,
        cancellation: InferenceCancellation,
        emit: &mut dyn FnMut(&NormalizedStreamEvent),
    ) -> Result<Vec<NormalizedStreamEvent>, LlamaCppAdapterError> {
        request.validate()?;
        if request.provider() != ModelProviderKind::LlamaCpp {
            return Err(LlamaCppAdapterError::WrongProvider);
        }

        let mut state = ModelStreamState::new(request, cancellation.clone());
        let mut events = Vec::new();
        push_event(
            &mut events,
            state.apply(0, ProviderStreamInput::Started)?,
            emit,
        );
        if cancellation.is_cancelled() {
            return Ok(events);
        }

        let deadline = Instant::now()
            .checked_add(Duration::from_millis(request.deadline_ms()))
            .ok_or(LlamaCppAdapterError::TimedOut)?;
        let payload = encode_request(request)?;
        let mut stream = match connect_checked(self.endpoint, deadline, &cancellation) {
            Ok(stream) => stream,
            Err(error) => return terminalize(error.into(), state, events, 1, emit),
        };
        if let Err(error) = configure_stream(&stream) {
            return terminalize(error.into(), state, events, 1, emit);
        }
        if let Err(error) = write_request(
            &mut stream,
            self.endpoint,
            LLAMA_CPP_PATH,
            "text/event-stream",
            &payload,
            deadline,
            &cancellation,
        ) {
            return terminalize(error.into(), state, events, 1, emit);
        }

        let mut reader = BufReader::new(stream);
        let framing = match read_headers(&mut reader, "text/event-stream", deadline, &cancellation)
        {
            Ok(framing) => framing,
            Err(error) => return terminalize(error.into(), state, events, 1, emit),
        };
        let mut decoder = SseDecoder::default();
        let mut sequence = 1;
        let mut stream_identity: Option<StreamIdentity> = None;
        let mut finish_reason = None;
        let mut done_seen = false;
        let mut calls = BTreeMap::new();
        let mut usage = None;

        let body_result = read_body(&mut reader, framing, deadline, &cancellation, |bytes| {
            for data in decoder.push(bytes).map_err(OllamaAdapterError::from)? {
                if sequence >= MAX_STREAM_EVENTS || done_seen {
                    return Err(OllamaAdapterError::BodyTooLarge);
                }
                if data == b"[DONE]" {
                    done_seen = true;
                    continue;
                }
                let chunk = parse_chunk(&data, request).map_err(OllamaAdapterError::from)?;
                bind_identity(&mut stream_identity, &chunk).map_err(OllamaAdapterError::from)?;
                if let Some(chunk_usage) = chunk.usage {
                    if usage.replace(chunk_usage).is_some() || !chunk.choices.is_empty() {
                        return Err(OllamaAdapterError::MalformedResponse);
                    }
                    continue;
                }
                if finish_reason.is_some() {
                    return Err(OllamaAdapterError::MalformedResponse);
                }
                if chunk.choices.len() != 1 {
                    return Err(OllamaAdapterError::MalformedResponse);
                }
                let choice = chunk
                    .choices
                    .into_iter()
                    .next()
                    .ok_or(OllamaAdapterError::MalformedResponse)?;
                if choice.index != 0 || choice.logprobs.is_some() {
                    return Err(OllamaAdapterError::MalformedResponse);
                }
                if choice
                    .delta
                    .role
                    .as_deref()
                    .is_some_and(|role| role != "assistant")
                    || choice.delta.reasoning_content.is_some()
                    || choice.delta.refusal.is_some()
                {
                    return Err(OllamaAdapterError::MalformedResponse);
                }
                if let Some(content) = choice.delta.content.filter(|value| !value.is_empty()) {
                    let event = state
                        .apply(sequence, ProviderStreamInput::TextDelta(content))
                        .map_err(OllamaAdapterError::from)?;
                    sequence += 1;
                    push_event(&mut events, event, emit);
                }
                for tool_call in choice.delta.tool_calls {
                    accumulate_tool_call(&mut calls, tool_call)
                        .map_err(OllamaAdapterError::from)?;
                }
                if let Some(reason) = choice.finish_reason
                    && finish_reason.replace(reason).is_some()
                {
                    return Err(OllamaAdapterError::MalformedResponse);
                }
            }
            Ok(())
        });

        if let Err(error) = body_result {
            return terminalize(error.into(), state, events, sequence, emit);
        }
        if cancellation.is_cancelled() {
            push_event(
                &mut events,
                state.apply(sequence, ProviderStreamInput::Finished)?,
                emit,
            );
            return Ok(events);
        }
        let trailing = match decoder.finish() {
            Ok(trailing) => trailing,
            Err(error) => return terminalize(error, state, events, sequence, emit),
        };
        if !trailing.is_empty() || !done_seen || stream_identity.is_none() {
            return terminalize(
                LlamaCppAdapterError::TruncatedResponse,
                state,
                events,
                sequence,
                emit,
            );
        }
        if let Err(error) = validate_finish_reason(finish_reason.as_deref(), calls.is_empty()) {
            return terminalize(error, state, events, sequence, emit);
        }
        if usage.as_ref().is_some_and(|usage| {
            usage.prompt_tokens.checked_add(usage.completion_tokens) != Some(usage.total_tokens)
        }) {
            return terminalize(
                LlamaCppAdapterError::MalformedResponse,
                state,
                events,
                sequence,
                emit,
            );
        }
        if !calls.is_empty() {
            let completion = match encode_tool_completion(calls) {
                Ok(completion) => completion,
                Err(error @ LlamaCppAdapterError::EncodingFailed) => return Err(error),
                Err(error) => return terminalize(error, state, events, sequence, emit),
            };
            push_event(
                &mut events,
                state.apply(sequence, ProviderStreamInput::ToolIntents(completion))?,
                emit,
            );
            sequence += 1;
        }
        if let Some(usage) = usage {
            push_event(
                &mut events,
                state.apply(
                    sequence,
                    ProviderStreamInput::Usage {
                        prompt_tokens: usage.prompt_tokens,
                        generated_tokens: usage.completion_tokens,
                    },
                )?,
                emit,
            );
            sequence += 1;
        }
        push_event(
            &mut events,
            state.apply(sequence, ProviderStreamInput::Finished)?,
            emit,
        );
        Ok(events)
    }
}

fn push_event(
    events: &mut Vec<NormalizedStreamEvent>,
    event: NormalizedStreamEvent,
    emit: &mut dyn FnMut(&NormalizedStreamEvent),
) {
    let index = events.len();
    events.push(event);
    emit(&events[index]);
}

fn terminalize(
    error: LlamaCppAdapterError,
    mut state: ModelStreamState,
    mut events: Vec<NormalizedStreamEvent>,
    sequence: u64,
    emit: &mut dyn FnMut(&NormalizedStreamEvent),
) -> Result<Vec<NormalizedStreamEvent>, LlamaCppAdapterError> {
    let category = match error {
        LlamaCppAdapterError::Cancelled => {
            push_event(
                &mut events,
                state.apply(sequence, ProviderStreamInput::Finished)?,
                emit,
            );
            return Ok(events);
        }
        LlamaCppAdapterError::TimedOut => ProviderFailureCategory::TimedOut,
        LlamaCppAdapterError::Unavailable => ProviderFailureCategory::Unavailable,
        LlamaCppAdapterError::Disconnected | LlamaCppAdapterError::TruncatedResponse => {
            ProviderFailureCategory::Disconnected
        }
        LlamaCppAdapterError::BodyTooLarge => ProviderFailureCategory::OutputLimit,
        LlamaCppAdapterError::HttpStatus => ProviderFailureCategory::ProviderFailed,
        LlamaCppAdapterError::HeaderTooLarge
        | LlamaCppAdapterError::MalformedHttp
        | LlamaCppAdapterError::UnsupportedHttpEncoding
        | LlamaCppAdapterError::MalformedResponse
        | LlamaCppAdapterError::EventAfterDone => ProviderFailureCategory::Malformed,
        LlamaCppAdapterError::Contract(_)
        | LlamaCppAdapterError::WrongProvider
        | LlamaCppAdapterError::InvalidEndpoint
        | LlamaCppAdapterError::EncodingFailed => return Err(error),
    };
    match state.apply(sequence, ProviderStreamInput::Failed(category)) {
        Ok(event) => {
            push_event(&mut events, event, emit);
            Ok(events)
        }
        Err(_) => Err(error),
    }
}

#[derive(Serialize)]
struct LlamaRequest<'a> {
    model: &'a str,
    messages: Vec<LlamaRequestMessage<'a>>,
    stream: bool,
    stream_options: StreamOptions,
    max_tokens: u32,
    temperature: u32,
    seed: u32,
    reasoning_effort: &'static str,
    reasoning_control: bool,
    chat_template_kwargs: ChatTemplateKwargs,
    parse_tool_calls: bool,
    parallel_tool_calls: bool,
    logprobs: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<LlamaTool>,
}

#[derive(Serialize)]
struct LlamaRequestMessage<'a> {
    role: &'static str,
    content: &'a str,
}

#[derive(Serialize)]
struct StreamOptions {
    include_usage: bool,
}

#[derive(Serialize)]
struct ChatTemplateKwargs {
    enable_thinking: bool,
}

#[derive(Serialize)]
struct LlamaTool {
    #[serde(rename = "type")]
    tool_type: &'static str,
    function: ModelIntentDefinition,
}

fn encode_request(request: &InferenceRequest) -> Result<Vec<u8>, LlamaCppAdapterError> {
    let messages = request
        .messages()
        .iter()
        .map(|message| LlamaRequestMessage {
            role: match message.role() {
                ConversationRole::System => "system",
                ConversationRole::User => "user",
                ConversationRole::Assistant => "assistant",
                ConversationRole::Tool => "tool",
            },
            content: message.content(),
        })
        .collect();
    let tools: Vec<_> = if request.output_mode() == InferenceOutputMode::BlossomTurn {
        request
            .intents()
            .iter()
            .map(|intent| LlamaTool {
                tool_type: "function",
                function: intent.definition(),
            })
            .collect()
    } else {
        Vec::new()
    };
    serde_json::to_vec(&LlamaRequest {
        model: request.model().as_str(),
        messages,
        stream: true,
        stream_options: StreamOptions {
            include_usage: true,
        },
        max_tokens: MAX_GENERATED_TOKENS,
        temperature: 0,
        seed: 0,
        reasoning_effort: "none",
        reasoning_control: false,
        chat_template_kwargs: ChatTemplateKwargs {
            enable_thinking: false,
        },
        parse_tool_calls: !tools.is_empty(),
        parallel_tool_calls: false,
        logprobs: false,
        tools,
    })
    .map_err(|_| LlamaCppAdapterError::EncodingFailed)
}

#[derive(Default)]
struct SseDecoder {
    pending: Vec<u8>,
}

impl SseDecoder {
    fn push(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, LlamaCppAdapterError> {
        self.pending.extend_from_slice(bytes);
        if self.pending.len() > MAX_SSE_EVENT_BYTES {
            return Err(LlamaCppAdapterError::BodyTooLarge);
        }
        let mut events = Vec::new();
        while let Some((position, delimiter_len)) = find_sse_delimiter(&self.pending) {
            let event: Vec<_> = self.pending.drain(..position).collect();
            self.pending.drain(..delimiter_len);
            if let Some(data) = parse_sse_event(&event)? {
                events.push(data);
            }
        }
        Ok(events)
    }

    fn finish(&mut self) -> Result<Vec<u8>, LlamaCppAdapterError> {
        if self.pending.len() > MAX_SSE_EVENT_BYTES {
            return Err(LlamaCppAdapterError::BodyTooLarge);
        }
        Ok(std::mem::take(&mut self.pending))
    }
}

fn find_sse_delimiter(bytes: &[u8]) -> Option<(usize, usize)> {
    bytes
        .windows(2)
        .position(|part| part == b"\n\n")
        .map(|position| (position, 2))
        .or_else(|| {
            bytes
                .windows(4)
                .position(|part| part == b"\r\n\r\n")
                .map(|position| (position, 4))
        })
}

fn parse_sse_event(event: &[u8]) -> Result<Option<Vec<u8>>, LlamaCppAdapterError> {
    let text = std::str::from_utf8(event).map_err(|_| LlamaCppAdapterError::MalformedResponse)?;
    let mut data = None;
    for line in text.lines() {
        if line.starts_with(':') {
            continue;
        }
        let value = line
            .strip_prefix("data: ")
            .ok_or(LlamaCppAdapterError::MalformedResponse)?;
        if value.is_empty() || data.replace(value.as_bytes().to_vec()).is_some() {
            return Err(LlamaCppAdapterError::MalformedResponse);
        }
    }
    Ok(data)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StreamIdentity {
    id: String,
    created: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChatChunk {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<ChatChoice>,
    #[serde(default)]
    usage: Option<Usage>,
    #[serde(default)]
    system_fingerprint: Option<String>,
    #[serde(default)]
    timings: Option<Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChatChoice {
    index: u32,
    delta: ChatDelta,
    finish_reason: Option<String>,
    #[serde(default)]
    logprobs: Option<Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChatDelta {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    refusal: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ToolCallDelta>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolCallDelta {
    index: u32,
    #[serde(default)]
    id: Option<String>,
    #[serde(rename = "type", default)]
    tool_type: Option<String>,
    function: FunctionDelta,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FunctionDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct Usage {
    completion_tokens: u64,
    prompt_tokens: u64,
    total_tokens: u64,
    #[serde(default)]
    prompt_tokens_details: Option<PromptTokenDetails>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct PromptTokenDetails {
    cached_tokens: u64,
}

fn parse_chunk(
    bytes: &[u8],
    request: &InferenceRequest,
) -> Result<ChatChunk, LlamaCppAdapterError> {
    let chunk: ChatChunk =
        serde_json::from_slice(bytes).map_err(|_| LlamaCppAdapterError::MalformedResponse)?;
    if chunk.id.is_empty()
        || chunk.id.len() > MAX_STREAM_ID_BYTES
        || chunk.id.bytes().any(|byte| byte.is_ascii_control())
        || chunk.object != "chat.completion.chunk"
        || chunk.model != request.model().as_str()
        || chunk.system_fingerprint.as_ref().is_some_and(|value| {
            value.len() > MAX_STREAM_ID_BYTES || value.bytes().any(|byte| byte.is_ascii_control())
        })
        || chunk.timings.as_ref().is_some_and(metadata_too_large)
        || chunk.usage.as_ref().is_some_and(|usage| {
            usage
                .prompt_tokens_details
                .is_some_and(|details| details.cached_tokens > usage.prompt_tokens)
        })
    {
        return Err(LlamaCppAdapterError::MalformedResponse);
    }
    Ok(chunk)
}

fn metadata_too_large(value: &Value) -> bool {
    serde_json::to_vec(value).map_or(true, |bytes| bytes.len() > MAX_METADATA_BYTES)
}

fn bind_identity(
    identity: &mut Option<StreamIdentity>,
    chunk: &ChatChunk,
) -> Result<(), LlamaCppAdapterError> {
    let observed = StreamIdentity {
        id: chunk.id.clone(),
        created: chunk.created,
    };
    match identity {
        Some(expected) if expected != &observed => Err(LlamaCppAdapterError::MalformedResponse),
        Some(_) => Ok(()),
        None => {
            *identity = Some(observed);
            Ok(())
        }
    }
}

#[derive(Default)]
struct PendingToolCall {
    id: Option<String>,
    name: String,
    arguments: String,
}

fn accumulate_tool_call(
    calls: &mut BTreeMap<u32, PendingToolCall>,
    delta: ToolCallDelta,
) -> Result<(), LlamaCppAdapterError> {
    if delta.index as usize >= super::MAX_TOOL_INTENTS
        || delta
            .tool_type
            .as_deref()
            .is_some_and(|value| value != "function")
        || delta.id.as_ref().is_some_and(|value| {
            value.is_empty()
                || value.len() > MAX_TOOL_CALL_ID_BYTES
                || value.bytes().any(|byte| byte.is_ascii_control())
        })
    {
        return Err(LlamaCppAdapterError::MalformedResponse);
    }
    let call = calls.entry(delta.index).or_default();
    if let Some(id) = delta.id {
        if call.id.as_ref().is_some_and(|existing| existing != &id) {
            return Err(LlamaCppAdapterError::MalformedResponse);
        }
        call.id = Some(id);
    }
    if let Some(name) = delta.function.name {
        call.name.push_str(&name);
    }
    if let Some(arguments) = delta.function.arguments {
        call.arguments.push_str(&arguments);
    }
    if call.name.len() > MAX_TOOL_NAME_BYTES
        || call.arguments.len() > MAX_TOOL_ARGUMENT_BYTES
        || call.name.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(LlamaCppAdapterError::BodyTooLarge);
    }
    Ok(())
}

fn validate_finish_reason(
    reason: Option<&str>,
    calls_empty: bool,
) -> Result<(), LlamaCppAdapterError> {
    match (reason, calls_empty) {
        (Some("stop" | "length"), true) | (Some("tool_calls"), false) => Ok(()),
        _ => Err(LlamaCppAdapterError::MalformedResponse),
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ToolCompletion {
    ToolIntents { intents: Vec<ToolIntent> },
}

#[derive(Serialize)]
struct ToolIntent {
    name: String,
    arguments: Value,
}

fn encode_tool_completion(
    calls: BTreeMap<u32, PendingToolCall>,
) -> Result<Vec<u8>, LlamaCppAdapterError> {
    let mut intents = Vec::with_capacity(calls.len());
    for (expected, (index, call)) in calls.into_iter().enumerate() {
        if index as usize != expected || call.name.is_empty() || call.arguments.is_empty() {
            return Err(LlamaCppAdapterError::MalformedResponse);
        }
        let arguments = serde_json::from_str(&call.arguments)
            .map_err(|_| LlamaCppAdapterError::MalformedResponse)?;
        intents.push(ToolIntent {
            name: call.name,
            arguments,
        });
    }
    serde_json::to_vec(&ToolCompletion::ToolIntents { intents })
        .map_err(|_| LlamaCppAdapterError::EncodingFailed)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LlamaCppAdapterError {
    Contract(ModelContractError),
    WrongProvider,
    InvalidEndpoint,
    EncodingFailed,
    Unavailable,
    TimedOut,
    Cancelled,
    Disconnected,
    BodyTooLarge,
    HeaderTooLarge,
    MalformedHttp,
    HttpStatus,
    UnsupportedHttpEncoding,
    MalformedResponse,
    TruncatedResponse,
    EventAfterDone,
}

impl From<ModelContractError> for LlamaCppAdapterError {
    fn from(error: ModelContractError) -> Self {
        Self::Contract(error)
    }
}

impl From<OllamaAdapterError> for LlamaCppAdapterError {
    fn from(error: OllamaAdapterError) -> Self {
        match error {
            OllamaAdapterError::Contract(error) => Self::Contract(error),
            OllamaAdapterError::WrongProvider => Self::WrongProvider,
            OllamaAdapterError::InvalidEndpoint => Self::InvalidEndpoint,
            OllamaAdapterError::EncodingFailed => Self::EncodingFailed,
            OllamaAdapterError::Unavailable => Self::Unavailable,
            OllamaAdapterError::HeaderTooLarge => Self::HeaderTooLarge,
            OllamaAdapterError::TimedOut => Self::TimedOut,
            OllamaAdapterError::Cancelled => Self::Cancelled,
            OllamaAdapterError::Disconnected => Self::Disconnected,
            OllamaAdapterError::BodyTooLarge => Self::BodyTooLarge,
            OllamaAdapterError::MalformedHttp => Self::MalformedHttp,
            OllamaAdapterError::HttpStatus => Self::HttpStatus,
            OllamaAdapterError::UnsupportedHttpEncoding => Self::UnsupportedHttpEncoding,
            OllamaAdapterError::MalformedResponse => Self::MalformedResponse,
            OllamaAdapterError::TruncatedResponse => Self::TruncatedResponse,
            OllamaAdapterError::EventAfterDone => Self::EventAfterDone,
        }
    }
}

impl From<LlamaCppAdapterError> for OllamaAdapterError {
    fn from(error: LlamaCppAdapterError) -> Self {
        match error {
            LlamaCppAdapterError::Contract(error) => Self::Contract(error),
            LlamaCppAdapterError::TimedOut => Self::TimedOut,
            LlamaCppAdapterError::Cancelled => Self::Cancelled,
            LlamaCppAdapterError::Disconnected => Self::Disconnected,
            LlamaCppAdapterError::BodyTooLarge => Self::BodyTooLarge,
            LlamaCppAdapterError::HeaderTooLarge => Self::HeaderTooLarge,
            LlamaCppAdapterError::HttpStatus => Self::HttpStatus,
            LlamaCppAdapterError::UnsupportedHttpEncoding => Self::UnsupportedHttpEncoding,
            LlamaCppAdapterError::TruncatedResponse => Self::TruncatedResponse,
            LlamaCppAdapterError::EventAfterDone => Self::EventAfterDone,
            LlamaCppAdapterError::WrongProvider
            | LlamaCppAdapterError::InvalidEndpoint
            | LlamaCppAdapterError::EncodingFailed
            | LlamaCppAdapterError::Unavailable
            | LlamaCppAdapterError::MalformedHttp
            | LlamaCppAdapterError::MalformedResponse => Self::MalformedResponse,
        }
    }
}

impl fmt::Display for LlamaCppAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Contract(_) => "llama.cpp response violated the model contract",
            Self::WrongProvider => "request selected a different provider",
            Self::InvalidEndpoint => "llama.cpp endpoint is not loopback",
            Self::EncodingFailed => "llama.cpp request encoding failed",
            Self::Unavailable => "llama.cpp transport is unavailable",
            Self::TimedOut => "llama.cpp request timed out",
            Self::Cancelled => "llama.cpp request was cancelled",
            Self::Disconnected => "llama.cpp disconnected before completion",
            Self::BodyTooLarge => "llama.cpp response exceeds its bound",
            Self::HeaderTooLarge => "llama.cpp response headers exceed their bound",
            Self::MalformedHttp => "llama.cpp returned malformed HTTP",
            Self::HttpStatus => "llama.cpp returned a non-success HTTP status",
            Self::UnsupportedHttpEncoding => "llama.cpp used an unsupported HTTP encoding",
            Self::MalformedResponse => "llama.cpp returned a malformed response",
            Self::TruncatedResponse => "llama.cpp response ended before completion",
            Self::EventAfterDone => "llama.cpp sent data after its terminal response",
        })
    }
}

impl std::error::Error for LlamaCppAdapterError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_runtime::{
        ConversationMessage, InferenceRequestId, ModelIntentKind, ModelProfile,
        NormalizedCompletion, NormalizedStreamKind, TurnIntentCatalogue,
    };
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpListener, TcpStream};
    use std::thread::{self, JoinHandle};

    fn request(intents: TurnIntentCatalogue, deadline_ms: u64) -> InferenceRequest {
        InferenceRequest::synthetic(
            InferenceRequestId::parse("llama-fixture-1".into()).unwrap(),
            ModelProviderKind::LlamaCpp,
            ModelProfile::parse("fixture-model:1".into()).unwrap(),
            vec![
                ConversationMessage::new(ConversationRole::User, "synthetic prompt".into())
                    .unwrap(),
            ],
            intents,
            InferenceOutputMode::BlossomTurn,
            deadline_ms,
        )
        .unwrap()
    }

    fn read_request(stream: &mut TcpStream) -> Vec<u8> {
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0; 1024];
        let header_end = loop {
            let count = stream.read(&mut buffer).unwrap();
            assert_ne!(count, 0);
            request.extend_from_slice(&buffer[..count]);
            if let Some(position) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                break position + 4;
            }
            assert!(request.len() <= super::super::ollama::MAX_HTTP_HEADER_BYTES);
        };
        let headers = std::str::from_utf8(&request[..header_end]).unwrap();
        let content_length = headers
            .lines()
            .find_map(|line| {
                line.strip_prefix("Content-Length: ")
                    .map(|value| value.parse::<usize>().unwrap())
            })
            .unwrap();
        while request.len() - header_end < content_length {
            let count = stream.read(&mut buffer).unwrap();
            assert_ne!(count, 0);
            request.extend_from_slice(&buffer[..count]);
        }
        request
    }

    fn server(parts: Vec<Vec<u8>>) -> (LlamaCppAdapter, JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let adapter = LlamaCppAdapter::for_test(listener.local_addr().unwrap()).unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            for part in parts {
                stream.write_all(&part).unwrap();
                thread::sleep(Duration::from_millis(5));
            }
            stream.shutdown(Shutdown::Write).unwrap();
            request
        });
        (adapter, handle)
    }

    fn response(body: &[u8]) -> Vec<Vec<u8>> {
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        vec![header.into_bytes(), body.to_vec()]
    }

    fn assert_malformed(body: &[u8]) {
        let (adapter, handle) = server(response(body));
        let events = adapter
            .infer(
                &request(TurnIntentCatalogue::empty(), 2_000),
                InferenceCancellation::new(),
            )
            .unwrap();
        assert!(matches!(
            events.last().unwrap().event,
            NormalizedStreamKind::Failed {
                category: ProviderFailureCategory::Malformed
            }
        ));
        handle.join().unwrap();
    }

    fn text_sse() -> Vec<u8> {
        concat!(
            "data: ",
            r#"{"id":"chatcmpl-1","object":"chat.completion.chunk","created":1,"model":"fixture-model:1","choices":[{"index":0,"delta":{"role":"assistant","content":"hel"},"finish_reason":null}]}"#,
            "\n\n",
            "data: ",
            r#"{"id":"chatcmpl-1","object":"chat.completion.chunk","created":1,"model":"fixture-model:1","choices":[{"index":0,"delta":{"content":"lo"},"finish_reason":"stop"}]}"#,
            "\n\n",
            "data: ",
            r#"{"id":"chatcmpl-1","object":"chat.completion.chunk","created":1,"model":"fixture-model:1","choices":[],"usage":{"completion_tokens":2,"prompt_tokens":3,"total_tokens":5,"prompt_tokens_details":{"cached_tokens":1}}}"#,
            "\n\n",
            "data: [DONE]\n\n"
        )
        .as_bytes()
        .to_vec()
    }

    #[test]
    fn fixed_request_and_fragmented_sse_normalize_text_deterministically() {
        let body = text_sse();
        let split = body.len() / 2;
        let mut wire = response(&body);
        let body = wire.pop().unwrap();
        wire.push(body[..split].to_vec());
        wire.push(body[split..].to_vec());
        let (adapter, handle) = server(wire);
        let mut events = Vec::new();
        adapter
            .stream(
                &request(TurnIntentCatalogue::empty(), 2_000),
                InferenceCancellation::new(),
                |event| events.push(event.clone()),
            )
            .unwrap();
        assert!(matches!(events[0].event, NormalizedStreamKind::Started));
        assert!(matches!(
            &events.last().unwrap().event,
            NormalizedStreamKind::Finished {
                completion: NormalizedCompletion::Text { content }
            } if content == "hello"
        ));
        assert!(events.iter().any(|event| matches!(
            event.event,
            NormalizedStreamKind::Usage {
                prompt_tokens: 3,
                generated_tokens: 2
            }
        )));

        let outbound = String::from_utf8(handle.join().unwrap()).unwrap();
        assert!(outbound.starts_with("POST /v1/chat/completions HTTP/1.1\r\nHost: 127.0.0.1:"));
        assert!(outbound.contains("\"reasoning_effort\":\"none\""));
        assert!(outbound.contains("\"enable_thinking\":false"));
        assert!(outbound.contains("\"parse_tool_calls\":false"));
        assert!(outbound.contains("\"parallel_tool_calls\":false"));
        assert!(!outbound.contains("image_url"));
        assert!(!outbound.contains("media"));
        assert!(!outbound.contains("/tools"));
    }

    #[test]
    fn fragmented_tool_arguments_become_only_one_validated_proposal() {
        let body = concat!(
            "data: ",
            r#"{"id":"chatcmpl-2","object":"chat.completion.chunk","created":2,"model":"fixture-model:1","choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"call-1","type":"function","function":{"name":"process.list","arguments":"{"}}]},"finish_reason":null}]}"#,
            "\n\n",
            "data: ",
            r#"{"id":"chatcmpl-2","object":"chat.completion.chunk","created":2,"model":"fixture-model:1","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"}"}}]},"finish_reason":"tool_calls"}]}"#,
            "\n\n",
            "data: [DONE]\n\n"
        );
        let (adapter, handle) = server(response(body.as_bytes()));
        let catalogue = TurnIntentCatalogue::from_eligible([ModelIntentKind::ProcessList]).unwrap();
        let events = adapter
            .infer(&request(catalogue, 2_000), InferenceCancellation::new())
            .unwrap();
        assert!(matches!(
            &events.last().unwrap().event,
            NormalizedStreamKind::Finished {
                completion: NormalizedCompletion::ToolIntents { intents }
            } if intents.len() == 1 && intents[0].kind() == ModelIntentKind::ProcessList
        ));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event.event, NormalizedStreamKind::TextDelta { .. }))
        );
        let outbound = String::from_utf8(handle.join().unwrap()).unwrap();
        assert!(outbound.contains("\"parse_tool_calls\":true"));
        assert!(outbound.contains("\"name\":\"process.list\""));
        assert!(!outbound.contains(".ssh"));
    }

    #[test]
    fn mixed_unlisted_reasoning_and_unknown_events_fail_closed() {
        let mixed = concat!(
            "data: ",
            r#"{"id":"chatcmpl-3","object":"chat.completion.chunk","created":3,"model":"fixture-model:1","choices":[{"index":0,"delta":{"content":"run","tool_calls":[{"index":0,"type":"function","function":{"name":"process.list","arguments":"{}"}}]},"finish_reason":"tool_calls"}]}"#,
            "\n\n",
            "data: [DONE]\n\n"
        );
        let (adapter, handle) = server(response(mixed.as_bytes()));
        let catalogue = TurnIntentCatalogue::from_eligible([ModelIntentKind::ProcessList]).unwrap();
        assert_eq!(
            adapter.infer(&request(catalogue, 2_000), InferenceCancellation::new()),
            Err(LlamaCppAdapterError::Contract(
                ModelContractError::MixedCompletion
            ))
        );
        handle.join().unwrap();

        let unlisted = concat!(
            "data: ",
            r#"{"id":"chatcmpl-4","object":"chat.completion.chunk","created":4,"model":"fixture-model:1","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"type":"function","function":{"name":"process.list","arguments":"{}"}}]},"finish_reason":"tool_calls"}]}"#,
            "\n\n",
            "data: [DONE]\n\n"
        );
        let (adapter, handle) = server(response(unlisted.as_bytes()));
        assert_eq!(
            adapter.infer(
                &request(TurnIntentCatalogue::empty(), 2_000),
                InferenceCancellation::new()
            ),
            Err(LlamaCppAdapterError::Contract(
                ModelContractError::IntentNotEligible
            ))
        );
        handle.join().unwrap();

        let reasoning = concat!(
            "data: ",
            r#"{"id":"chatcmpl-5","object":"chat.completion.chunk","created":5,"model":"fixture-model:1","choices":[{"index":0,"delta":{"reasoning_content":"private trace"},"finish_reason":"stop"}]}"#,
            "\n\n",
            "data: [DONE]\n\n"
        );
        let (adapter, handle) = server(response(reasoning.as_bytes()));
        let events = adapter
            .infer(
                &request(TurnIntentCatalogue::empty(), 2_000),
                InferenceCancellation::new(),
            )
            .unwrap();
        assert!(matches!(
            events.last().unwrap().event,
            NormalizedStreamKind::Failed {
                category: ProviderFailureCategory::Malformed
            }
        ));
        handle.join().unwrap();

        let (adapter, handle) = server(response(b"event: authority\n\ndata: [DONE]\n\n"));
        let events = adapter
            .infer(
                &request(TurnIntentCatalogue::empty(), 2_000),
                InferenceCancellation::new(),
            )
            .unwrap();
        assert!(matches!(
            events.last().unwrap().event,
            NormalizedStreamKind::Failed {
                category: ProviderFailureCategory::Malformed
            }
        ));
        handle.join().unwrap();
    }

    #[test]
    fn terminal_metadata_identity_and_utf8_violations_fail_closed() {
        let after_finish = concat!(
            "data: ",
            r#"{"id":"chatcmpl-6","object":"chat.completion.chunk","created":6,"model":"fixture-model:1","choices":[{"index":0,"delta":{"content":"done"},"finish_reason":"stop"}]}"#,
            "\n\n",
            "data: ",
            r#"{"id":"chatcmpl-6","object":"chat.completion.chunk","created":6,"model":"fixture-model:1","choices":[{"index":0,"delta":{"content":"late"},"finish_reason":null}]}"#,
            "\n\n",
            "data: [DONE]\n\n"
        );
        assert_malformed(after_finish.as_bytes());

        let usage_overflow = concat!(
            "data: ",
            r#"{"id":"chatcmpl-7","object":"chat.completion.chunk","created":7,"model":"fixture-model:1","choices":[{"index":0,"delta":{"content":"done"},"finish_reason":"stop"}]}"#,
            "\n\n",
            "data: ",
            r#"{"id":"chatcmpl-7","object":"chat.completion.chunk","created":7,"model":"fixture-model:1","choices":[],"usage":{"completion_tokens":18446744073709551615,"prompt_tokens":1,"total_tokens":18446744073709551615}}"#,
            "\n\n",
            "data: [DONE]\n\n"
        );
        assert_malformed(usage_overflow.as_bytes());

        let changed_identity = concat!(
            "data: ",
            r#"{"id":"chatcmpl-8","object":"chat.completion.chunk","created":8,"model":"fixture-model:1","choices":[{"index":0,"delta":{"content":"a"},"finish_reason":null}]}"#,
            "\n\n",
            "data: ",
            r#"{"id":"chatcmpl-other","object":"chat.completion.chunk","created":8,"model":"fixture-model:1","choices":[{"index":0,"delta":{"content":"b"},"finish_reason":"stop"}]}"#,
            "\n\n",
            "data: [DONE]\n\n"
        );
        assert_malformed(changed_identity.as_bytes());

        let mut invalid_utf8 = b"data: {\"id\":\"chatcmpl-9\"".to_vec();
        invalid_utf8.push(0xff);
        invalid_utf8.extend_from_slice(b"}\n\ndata: [DONE]\n\n");
        assert_malformed(&invalid_utf8);
    }

    #[test]
    fn cancellation_deadline_redirect_and_non_loopback_are_bounded() {
        let cancellation = InferenceCancellation::new();
        cancellation.cancel();
        let events = LlamaCppAdapter::default()
            .infer(&request(TurnIntentCatalogue::empty(), 2_000), cancellation)
            .unwrap();
        assert!(matches!(
            events.last().unwrap().event,
            NormalizedStreamKind::Cancelled
        ));

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let adapter = LlamaCppAdapter::for_test(listener.local_addr().unwrap()).unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            thread::sleep(Duration::from_millis(250));
            request
        });
        let events = adapter
            .infer(
                &request(TurnIntentCatalogue::empty(), 40),
                InferenceCancellation::new(),
            )
            .unwrap();
        assert!(matches!(
            events.last().unwrap().event,
            NormalizedStreamKind::Failed {
                category: ProviderFailureCategory::TimedOut
            }
        ));
        handle.join().unwrap();

        let (adapter, handle) = server(vec![
            b"HTTP/1.1 307 Redirect\r\nLocation: https://example.invalid/\r\nContent-Length: 0\r\n\r\n"
                .to_vec(),
        ]);
        let events = adapter
            .infer(
                &request(TurnIntentCatalogue::empty(), 2_000),
                InferenceCancellation::new(),
            )
            .unwrap();
        assert!(matches!(
            events.last().unwrap().event,
            NormalizedStreamKind::Failed {
                category: ProviderFailureCategory::ProviderFailed
            }
        ));
        handle.join().unwrap();

        assert!(matches!(
            LlamaCppAdapter::for_test("192.0.2.1:8080".parse().unwrap()),
            Err(LlamaCppAdapterError::InvalidEndpoint)
        ));
        assert_eq!(
            LlamaCppAdapter::default().endpoint.to_string(),
            LLAMA_CPP_ENDPOINT
        );
    }
}
