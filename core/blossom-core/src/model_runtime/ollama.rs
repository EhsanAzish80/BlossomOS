//! Fixed-loopback Ollama development adapter.
//!
//! This transport is intentionally synthetic-only because `InferenceRequest`
//! has no public constructor and its only internal constructor assigns the
//! `Synthetic` classification. The endpoint is code-owned and numeric, so this
//! module performs no DNS lookup, proxy discovery, redirect, TLS, or provider
//! lifecycle operation.

use super::{
    InferenceCancellation, InferenceOutputMode, InferenceRequest, ModelContractError,
    ModelIntentDefinition, ModelProviderKind, ModelStreamState, NormalizedStreamEvent,
    ProviderFailureCategory, ProviderStreamInput,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
use std::time::{Duration, Instant};

pub const OLLAMA_ENDPOINT: &str = "127.0.0.1:11434";
const OLLAMA_PATH: &str = "/api/chat";
pub(super) const MAX_HTTP_HEADER_BYTES: usize = 16 * 1024;
const MAX_HTTP_HEADERS: usize = 64;
pub(super) const MAX_HTTP_BODY_BYTES: usize = 256 * 1024;
const MAX_HTTP_CHUNK_BYTES: usize = 64 * 1024;
const MAX_PROVIDER_LINE_BYTES: usize = 192 * 1024;
const MAX_PROVIDER_EVENTS: u64 = 4_096;
const IO_POLL_INTERVAL: Duration = Duration::from_millis(100);
const MAX_GENERATED_TOKENS: u32 = 512;

#[derive(Clone, Copy, Debug)]
pub struct OllamaAdapter {
    endpoint: SocketAddr,
}

impl Default for OllamaAdapter {
    fn default() -> Self {
        Self {
            endpoint: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 11_434)),
        }
    }
}

impl OllamaAdapter {
    #[cfg(test)]
    pub(super) fn for_test(endpoint: SocketAddr) -> Result<Self, OllamaAdapterError> {
        if !endpoint.ip().is_loopback() {
            return Err(OllamaAdapterError::InvalidEndpoint);
        }
        Ok(Self { endpoint })
    }

    pub fn stream(
        &self,
        request: &InferenceRequest,
        cancellation: InferenceCancellation,
        mut emit: impl FnMut(&NormalizedStreamEvent),
    ) -> Result<(), OllamaAdapterError> {
        self.infer_inner(request, cancellation, &mut emit)
            .map(|_| ())
    }

    #[cfg(test)]
    pub(super) fn infer(
        &self,
        request: &InferenceRequest,
        cancellation: InferenceCancellation,
    ) -> Result<Vec<NormalizedStreamEvent>, OllamaAdapterError> {
        self.infer_inner(request, cancellation, &mut |_| {})
    }

    fn infer_inner(
        &self,
        request: &InferenceRequest,
        cancellation: InferenceCancellation,
        emit: &mut dyn FnMut(&NormalizedStreamEvent),
    ) -> Result<Vec<NormalizedStreamEvent>, OllamaAdapterError> {
        request.validate()?;
        if request.provider() != ModelProviderKind::Ollama {
            return Err(OllamaAdapterError::WrongProvider);
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
            .ok_or(OllamaAdapterError::TimedOut)?;
        let payload = encode_request(request)?;
        let remaining = remaining(deadline)?;
        let mut stream = match TcpStream::connect_timeout(&self.endpoint, remaining) {
            Ok(stream) => stream,
            Err(error) => {
                let category = if error.kind() == io::ErrorKind::TimedOut {
                    ProviderFailureCategory::TimedOut
                } else {
                    ProviderFailureCategory::Unavailable
                };
                push_event(
                    &mut events,
                    state.apply(1, ProviderStreamInput::Failed(category))?,
                    emit,
                );
                return Ok(events);
            }
        };
        if let Err(error) = configure_stream(&stream) {
            return terminalize(error, state, events, 1, emit);
        }
        match write_request(
            &mut stream,
            self.endpoint,
            OLLAMA_PATH,
            "application/x-ndjson",
            &payload,
            deadline,
            &cancellation,
        ) {
            Ok(()) => {}
            Err(OllamaAdapterError::Cancelled) => {
                push_event(
                    &mut events,
                    state.apply(1, ProviderStreamInput::Finished)?,
                    emit,
                );
                return Ok(events);
            }
            Err(error) => return terminalize(error, state, events, 1, emit),
        }

        let mut reader = BufReader::new(stream);
        let framing =
            match read_headers(&mut reader, "application/x-ndjson", deadline, &cancellation) {
                Ok(framing) => framing,
                Err(OllamaAdapterError::Cancelled) => {
                    push_event(
                        &mut events,
                        state.apply(1, ProviderStreamInput::Finished)?,
                        emit,
                    );
                    return Ok(events);
                }
                Err(error) => return terminalize(error, state, events, 1, emit),
            };
        let mut line_decoder = LineDecoder::default();
        let mut sequence = 1;
        let mut terminal_seen = false;
        let mut pending_intents = Vec::new();
        let mut usage = None;

        let body_result = read_body(&mut reader, framing, deadline, &cancellation, |bytes| {
            for line in line_decoder.push(bytes)? {
                if sequence >= MAX_PROVIDER_EVENTS {
                    return Err(OllamaAdapterError::BodyTooLarge);
                }
                if terminal_seen {
                    return Err(OllamaAdapterError::EventAfterDone);
                }
                let mut response = parse_response(&line, request)?;
                if !response.message.content.is_empty() {
                    let content = std::mem::take(&mut response.message.content);
                    let event = state.apply(sequence, ProviderStreamInput::TextDelta(content))?;
                    sequence += 1;
                    push_event(&mut events, event, emit);
                }
                pending_intents.append(&mut response.message.tool_calls);
                if response.done {
                    validate_done_reason(response.done_reason.as_deref())?;
                    usage = token_usage(&response)?;
                    terminal_seen = true;
                } else if response.done_reason.is_some() || has_terminal_metadata(&response) {
                    return Err(OllamaAdapterError::MalformedResponse);
                }
            }
            Ok(())
        });

        if body_result == Err(OllamaAdapterError::Cancelled) {
            push_event(
                &mut events,
                state.apply(sequence, ProviderStreamInput::Finished)?,
                emit,
            );
            return Ok(events);
        }
        if let Err(error) = body_result {
            return terminalize(error, state, events, sequence, emit);
        }

        if cancellation.is_cancelled() {
            push_event(
                &mut events,
                state.apply(sequence, ProviderStreamInput::Finished)?,
                emit,
            );
            return Ok(events);
        }
        let pending = match line_decoder.finish() {
            Ok(pending) => pending,
            Err(error) => return terminalize(error, state, events, sequence, emit),
        };
        if !pending.is_empty() {
            return terminalize(
                OllamaAdapterError::TruncatedResponse,
                state,
                events,
                sequence,
                emit,
            );
        }
        if !terminal_seen {
            return terminalize(
                OllamaAdapterError::TruncatedResponse,
                state,
                events,
                sequence,
                emit,
            );
        }
        if !pending_intents.is_empty() {
            let completion = encode_tool_completion(pending_intents)?;
            push_event(
                &mut events,
                state.apply(sequence, ProviderStreamInput::ToolIntents(completion))?,
                emit,
            );
            sequence += 1;
        }
        if let Some((prompt_tokens, generated_tokens)) = usage {
            push_event(
                &mut events,
                state.apply(
                    sequence,
                    ProviderStreamInput::Usage {
                        prompt_tokens,
                        generated_tokens,
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
    error: OllamaAdapterError,
    mut state: ModelStreamState,
    mut events: Vec<NormalizedStreamEvent>,
    sequence: u64,
    emit: &mut dyn FnMut(&NormalizedStreamEvent),
) -> Result<Vec<NormalizedStreamEvent>, OllamaAdapterError> {
    let category = match error {
        OllamaAdapterError::Cancelled => {
            push_event(
                &mut events,
                state.apply(sequence, ProviderStreamInput::Finished)?,
                emit,
            );
            return Ok(events);
        }
        OllamaAdapterError::TimedOut => ProviderFailureCategory::TimedOut,
        OllamaAdapterError::Unavailable => ProviderFailureCategory::Unavailable,
        OllamaAdapterError::Disconnected | OllamaAdapterError::TruncatedResponse => {
            ProviderFailureCategory::Disconnected
        }
        OllamaAdapterError::BodyTooLarge => ProviderFailureCategory::OutputLimit,
        OllamaAdapterError::HttpStatus => ProviderFailureCategory::ProviderFailed,
        OllamaAdapterError::HeaderTooLarge
        | OllamaAdapterError::MalformedHttp
        | OllamaAdapterError::UnsupportedHttpEncoding
        | OllamaAdapterError::MalformedResponse
        | OllamaAdapterError::EventAfterDone => ProviderFailureCategory::Malformed,
        OllamaAdapterError::Contract(_)
        | OllamaAdapterError::WrongProvider
        | OllamaAdapterError::InvalidEndpoint
        | OllamaAdapterError::EncodingFailed => return Err(error),
    };
    match state.apply(sequence, ProviderStreamInput::Failed(category)) {
        Ok(event) => {
            push_event(&mut events, event, emit);
            Ok(events)
        }
        Err(_) => Err(error),
    }
}

pub(super) fn configure_stream(stream: &TcpStream) -> Result<(), OllamaAdapterError> {
    stream.set_read_timeout(Some(IO_POLL_INTERVAL))?;
    stream.set_write_timeout(Some(IO_POLL_INTERVAL))?;
    stream.set_nodelay(true)?;
    Ok(())
}

#[derive(Serialize)]
struct OllamaRequest<'a> {
    model: &'a str,
    messages: Vec<OllamaRequestMessage<'a>>,
    stream: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<OllamaTool>,
    options: OllamaOptions,
    think: bool,
    logprobs: bool,
}

#[derive(Serialize)]
struct OllamaRequestMessage<'a> {
    role: &'static str,
    content: &'a str,
}

#[derive(Serialize)]
struct OllamaTool {
    #[serde(rename = "type")]
    tool_type: &'static str,
    function: ModelIntentDefinition,
}

#[derive(Serialize)]
struct OllamaOptions {
    seed: u32,
    temperature: u32,
    num_predict: u32,
}

fn encode_request(request: &InferenceRequest) -> Result<Vec<u8>, OllamaAdapterError> {
    let messages = request
        .messages()
        .iter()
        .map(|message| OllamaRequestMessage {
            role: match message.role() {
                super::ConversationRole::System => "system",
                super::ConversationRole::User => "user",
                super::ConversationRole::Assistant => "assistant",
                super::ConversationRole::Tool => "tool",
            },
            content: message.content(),
        })
        .collect();
    let tools = if request.output_mode() == InferenceOutputMode::BlossomTurn {
        request
            .intents()
            .iter()
            .map(|intent| OllamaTool {
                tool_type: "function",
                function: intent.definition(),
            })
            .collect()
    } else {
        Vec::new()
    };
    serde_json::to_vec(&OllamaRequest {
        model: request.model().as_str(),
        messages,
        stream: true,
        tools,
        options: OllamaOptions {
            seed: 0,
            temperature: 0,
            num_predict: MAX_GENERATED_TOKENS,
        },
        think: false,
        logprobs: false,
    })
    .map_err(|_| OllamaAdapterError::EncodingFailed)
}

pub(super) fn write_request(
    stream: &mut TcpStream,
    endpoint: SocketAddr,
    path: &str,
    accept: &str,
    payload: &[u8],
    deadline: Instant,
    cancellation: &InferenceCancellation,
) -> Result<(), OllamaAdapterError> {
    let header = format!(
        "POST {path} HTTP/1.1\r\nHost: {endpoint}\r\nContent-Type: application/json\r\nAccept: {accept}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    );
    write_all_checked(stream, header.as_bytes(), deadline, cancellation)?;
    write_all_checked(stream, payload, deadline, cancellation)
}

fn write_all_checked(
    stream: &mut TcpStream,
    bytes: &[u8],
    deadline: Instant,
    cancellation: &InferenceCancellation,
) -> Result<(), OllamaAdapterError> {
    let mut written = 0;
    while written < bytes.len() {
        check_progress(deadline, cancellation)?;
        match stream.write(&bytes[written..]) {
            Ok(0) => return Err(OllamaAdapterError::Disconnected),
            Ok(count) => written += count,
            Err(error) if is_retryable(&error) => {}
            Err(_) => return Err(OllamaAdapterError::Disconnected),
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) enum BodyFraming {
    ContentLength(usize),
    Chunked,
    UntilClose,
}

pub(super) fn read_headers(
    reader: &mut BufReader<TcpStream>,
    expected_content_type: &str,
    deadline: Instant,
    cancellation: &InferenceCancellation,
) -> Result<BodyFraming, OllamaAdapterError> {
    let status = read_line_checked(reader, deadline, cancellation, MAX_HTTP_HEADER_BYTES)?;
    let status = std::str::from_utf8(&status).map_err(|_| OllamaAdapterError::MalformedHttp)?;
    let mut status_parts = status.trim_end_matches("\r\n").splitn(3, ' ');
    let version = status_parts.next();
    let code = status_parts.next();
    let reason = status_parts.next();
    if !matches!(version, Some("HTTP/1.1" | "HTTP/1.0")) || reason.is_none_or(str::is_empty) {
        return Err(OllamaAdapterError::MalformedHttp);
    }
    if code != Some("200") {
        return Err(OllamaAdapterError::HttpStatus);
    }

    let mut total = status.len();
    let mut count = 0;
    let mut content_length = None;
    let mut chunked = false;
    let mut content_type_seen = false;
    loop {
        let line = read_line_checked(reader, deadline, cancellation, MAX_HTTP_HEADER_BYTES)?;
        total = total
            .checked_add(line.len())
            .ok_or(OllamaAdapterError::HeaderTooLarge)?;
        if total > MAX_HTTP_HEADER_BYTES {
            return Err(OllamaAdapterError::HeaderTooLarge);
        }
        if line == b"\r\n" {
            break;
        }
        count += 1;
        if count > MAX_HTTP_HEADERS || line.starts_with(b" ") || line.starts_with(b"\t") {
            return Err(OllamaAdapterError::MalformedHttp);
        }
        let text = std::str::from_utf8(&line).map_err(|_| OllamaAdapterError::MalformedHttp)?;
        let (name, value) = text
            .trim_end_matches("\r\n")
            .split_once(':')
            .ok_or(OllamaAdapterError::MalformedHttp)?;
        let name = name.to_ascii_lowercase();
        let value = value.trim();
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            || value.bytes().any(|byte| byte.is_ascii_control())
        {
            return Err(OllamaAdapterError::MalformedHttp);
        }
        match name.as_str() {
            "content-length" => {
                if content_length.is_some() {
                    return Err(OllamaAdapterError::MalformedHttp);
                }
                if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err(OllamaAdapterError::MalformedHttp);
                }
                let length = value
                    .parse::<usize>()
                    .map_err(|_| OllamaAdapterError::MalformedHttp)?;
                if length > MAX_HTTP_BODY_BYTES {
                    return Err(OllamaAdapterError::BodyTooLarge);
                }
                content_length = Some(length);
            }
            "transfer-encoding" => {
                if !value.eq_ignore_ascii_case("chunked") || chunked {
                    return Err(OllamaAdapterError::UnsupportedHttpEncoding);
                }
                chunked = true;
            }
            "content-encoding" if !value.eq_ignore_ascii_case("identity") => {
                return Err(OllamaAdapterError::UnsupportedHttpEncoding);
            }
            "content-type" => {
                if content_type_seen || !value.eq_ignore_ascii_case(expected_content_type) {
                    return Err(OllamaAdapterError::UnsupportedHttpEncoding);
                }
                content_type_seen = true;
            }
            "location" => return Err(OllamaAdapterError::MalformedHttp),
            _ => {}
        }
    }
    if !content_type_seen {
        return Err(OllamaAdapterError::MalformedHttp);
    }
    match (chunked, content_length) {
        (true, Some(_)) => Err(OllamaAdapterError::MalformedHttp),
        (true, None) => Ok(BodyFraming::Chunked),
        (false, Some(length)) => Ok(BodyFraming::ContentLength(length)),
        (false, None) => Ok(BodyFraming::UntilClose),
    }
}

pub(super) fn read_body(
    reader: &mut BufReader<TcpStream>,
    framing: BodyFraming,
    deadline: Instant,
    cancellation: &InferenceCancellation,
    mut consume: impl FnMut(&[u8]) -> Result<(), OllamaAdapterError>,
) -> Result<(), OllamaAdapterError> {
    let mut total = 0usize;
    match framing {
        BodyFraming::ContentLength(length) => {
            let mut remaining = length;
            while remaining > 0 {
                let chunk = read_some_checked(reader, remaining.min(8192), deadline, cancellation)?;
                remaining -= chunk.len();
                total += chunk.len();
                consume(&chunk)?;
            }
        }
        BodyFraming::UntilClose => {
            while let Some(chunk) = read_some_or_eof(reader, 8192, deadline, cancellation)? {
                total = bounded_total(total, chunk.len())?;
                consume(&chunk)?;
            }
        }
        BodyFraming::Chunked => loop {
            let size_line = read_line_checked(reader, deadline, cancellation, 128)?;
            let size_text = std::str::from_utf8(&size_line)
                .map_err(|_| OllamaAdapterError::MalformedHttp)?
                .trim_end_matches("\r\n");
            if size_text.contains(';') {
                return Err(OllamaAdapterError::UnsupportedHttpEncoding);
            }
            let size = usize::from_str_radix(size_text, 16)
                .map_err(|_| OllamaAdapterError::MalformedHttp)?;
            if size == 0 {
                let end = read_line_checked(reader, deadline, cancellation, 2)?;
                if end != b"\r\n" {
                    return Err(OllamaAdapterError::UnsupportedHttpEncoding);
                }
                break;
            }
            if size > MAX_HTTP_CHUNK_BYTES {
                return Err(OllamaAdapterError::BodyTooLarge);
            }
            total = bounded_total(total, size)?;
            let chunk = read_exact_checked(reader, size, deadline, cancellation)?;
            consume(&chunk)?;
            let end = read_exact_checked(reader, 2, deadline, cancellation)?;
            if end != b"\r\n" {
                return Err(OllamaAdapterError::MalformedHttp);
            }
        },
    }
    if total > MAX_HTTP_BODY_BYTES {
        return Err(OllamaAdapterError::BodyTooLarge);
    }
    Ok(())
}

fn bounded_total(total: usize, added: usize) -> Result<usize, OllamaAdapterError> {
    let total = total
        .checked_add(added)
        .ok_or(OllamaAdapterError::BodyTooLarge)?;
    if total > MAX_HTTP_BODY_BYTES {
        return Err(OllamaAdapterError::BodyTooLarge);
    }
    Ok(total)
}

fn read_line_checked(
    reader: &mut BufReader<TcpStream>,
    deadline: Instant,
    cancellation: &InferenceCancellation,
    bound: usize,
) -> Result<Vec<u8>, OllamaAdapterError> {
    let mut line = Vec::new();
    loop {
        check_progress(deadline, cancellation)?;
        match reader.fill_buf() {
            Ok([]) => return Err(OllamaAdapterError::Disconnected),
            Ok(buffer) => {
                let take = buffer
                    .iter()
                    .position(|byte| *byte == b'\n')
                    .map_or(buffer.len(), |position| position + 1);
                if line.len().saturating_add(take) > bound {
                    return Err(OllamaAdapterError::HeaderTooLarge);
                }
                let complete = buffer[take - 1] == b'\n';
                line.extend_from_slice(&buffer[..take]);
                reader.consume(take);
                if complete {
                    return line
                        .ends_with(b"\r\n")
                        .then_some(line)
                        .ok_or(OllamaAdapterError::MalformedHttp);
                }
            }
            Err(error) if is_retryable(&error) => {}
            Err(_) => return Err(OllamaAdapterError::Disconnected),
        }
    }
}

fn read_exact_checked(
    reader: &mut BufReader<TcpStream>,
    length: usize,
    deadline: Instant,
    cancellation: &InferenceCancellation,
) -> Result<Vec<u8>, OllamaAdapterError> {
    let mut bytes = vec![0; length];
    let mut read = 0;
    while read < length {
        check_progress(deadline, cancellation)?;
        match reader.read(&mut bytes[read..]) {
            Ok(0) => return Err(OllamaAdapterError::Disconnected),
            Ok(count) => read += count,
            Err(error) if is_retryable(&error) => {}
            Err(_) => return Err(OllamaAdapterError::Disconnected),
        }
    }
    Ok(bytes)
}

fn read_some_checked(
    reader: &mut BufReader<TcpStream>,
    maximum: usize,
    deadline: Instant,
    cancellation: &InferenceCancellation,
) -> Result<Vec<u8>, OllamaAdapterError> {
    read_some_or_eof(reader, maximum, deadline, cancellation)?
        .ok_or(OllamaAdapterError::Disconnected)
}

fn read_some_or_eof(
    reader: &mut BufReader<TcpStream>,
    maximum: usize,
    deadline: Instant,
    cancellation: &InferenceCancellation,
) -> Result<Option<Vec<u8>>, OllamaAdapterError> {
    let mut bytes = vec![0; maximum];
    loop {
        check_progress(deadline, cancellation)?;
        match reader.read(&mut bytes) {
            Ok(0) => return Ok(None),
            Ok(count) => {
                bytes.truncate(count);
                return Ok(Some(bytes));
            }
            Err(error) if is_retryable(&error) => {}
            Err(_) => return Err(OllamaAdapterError::Disconnected),
        }
    }
}

fn check_progress(
    deadline: Instant,
    cancellation: &InferenceCancellation,
) -> Result<(), OllamaAdapterError> {
    if cancellation.is_cancelled() {
        return Err(OllamaAdapterError::Cancelled);
    }
    if Instant::now() >= deadline {
        return Err(OllamaAdapterError::TimedOut);
    }
    Ok(())
}

pub(super) fn remaining(deadline: Instant) -> Result<Duration, OllamaAdapterError> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or(OllamaAdapterError::TimedOut)
}

fn is_retryable(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
    )
}

#[derive(Default)]
struct LineDecoder {
    pending: Vec<u8>,
}

impl LineDecoder {
    fn push(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, OllamaAdapterError> {
        self.pending.extend_from_slice(bytes);
        if self.pending.len() > MAX_PROVIDER_LINE_BYTES {
            return Err(OllamaAdapterError::BodyTooLarge);
        }
        let mut lines = Vec::new();
        while let Some(position) = self.pending.iter().position(|byte| *byte == b'\n') {
            let mut line: Vec<_> = self.pending.drain(..=position).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            if line.is_empty() {
                return Err(OllamaAdapterError::MalformedResponse);
            }
            lines.push(line);
        }
        Ok(lines)
    }

    fn finish(&mut self) -> Result<Vec<u8>, OllamaAdapterError> {
        if self.pending.len() > MAX_PROVIDER_LINE_BYTES {
            return Err(OllamaAdapterError::BodyTooLarge);
        }
        Ok(std::mem::take(&mut self.pending))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OllamaResponse {
    model: String,
    created_at: String,
    message: OllamaMessage,
    done: bool,
    #[serde(default)]
    done_reason: Option<String>,
    #[serde(default)]
    total_duration: Option<u64>,
    #[serde(default)]
    load_duration: Option<u64>,
    #[serde(default)]
    prompt_eval_count: Option<u64>,
    #[serde(default)]
    prompt_eval_cached_count: Option<u64>,
    #[serde(default)]
    prompt_eval_duration: Option<u64>,
    #[serde(default)]
    eval_count: Option<u64>,
    #[serde(default)]
    eval_duration: Option<u64>,
    #[serde(default)]
    logprobs: Option<Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OllamaMessage {
    role: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    thinking: Option<String>,
    #[serde(default)]
    images: Vec<String>,
    #[serde(default)]
    tool_calls: Vec<OllamaToolCall>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OllamaToolCall {
    function: OllamaFunctionCall,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OllamaFunctionCall {
    name: String,
    #[serde(default, skip_serializing)]
    description: Option<String>,
    arguments: Value,
}

fn parse_response(
    line: &[u8],
    request: &InferenceRequest,
) -> Result<OllamaResponse, OllamaAdapterError> {
    let response: OllamaResponse =
        serde_json::from_slice(line).map_err(|_| OllamaAdapterError::MalformedResponse)?;
    if response.model != request.model().as_str()
        || response.created_at.is_empty()
        || response.created_at.len() > 64
        || response
            .created_at
            .bytes()
            .any(|byte| byte.is_ascii_control())
        || response.message.role != "assistant"
        || response.message.content.contains('\0')
        || response.message.thinking.is_some()
        || !response.message.images.is_empty()
        || response.logprobs.is_some()
        || response.message.tool_calls.iter().any(|call| {
            call.function.description.as_ref().is_some_and(|value| {
                value.len() > 512 || value.bytes().any(|byte| byte.is_ascii_control())
            })
        })
    {
        return Err(OllamaAdapterError::MalformedResponse);
    }
    Ok(response)
}

fn validate_done_reason(reason: Option<&str>) -> Result<(), OllamaAdapterError> {
    match reason {
        Some("stop" | "length") => Ok(()),
        _ => Err(OllamaAdapterError::MalformedResponse),
    }
}

fn has_terminal_metadata(response: &OllamaResponse) -> bool {
    response.total_duration.is_some()
        || response.load_duration.is_some()
        || response.prompt_eval_count.is_some()
        || response.prompt_eval_cached_count.is_some()
        || response.prompt_eval_duration.is_some()
        || response.eval_count.is_some()
        || response.eval_duration.is_some()
}

fn token_usage(response: &OllamaResponse) -> Result<Option<(u64, u64)>, OllamaAdapterError> {
    match (response.prompt_eval_count, response.eval_count) {
        (Some(prompt), Some(generated)) => Ok(Some((prompt, generated))),
        (None, None) => Ok(None),
        _ => Err(OllamaAdapterError::MalformedResponse),
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ToolCompletion {
    ToolIntents { intents: Vec<OllamaFunctionCall> },
}

fn encode_tool_completion(calls: Vec<OllamaToolCall>) -> Result<Vec<u8>, OllamaAdapterError> {
    let intents = calls.into_iter().map(|call| call.function).collect();
    serde_json::to_vec(&ToolCompletion::ToolIntents { intents })
        .map_err(|_| OllamaAdapterError::EncodingFailed)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OllamaAdapterError {
    Contract(ModelContractError),
    WrongProvider,
    InvalidEndpoint,
    EncodingFailed,
    Unavailable,
    TimedOut,
    Cancelled,
    Disconnected,
    HeaderTooLarge,
    BodyTooLarge,
    MalformedHttp,
    HttpStatus,
    UnsupportedHttpEncoding,
    MalformedResponse,
    TruncatedResponse,
    EventAfterDone,
}

impl From<ModelContractError> for OllamaAdapterError {
    fn from(error: ModelContractError) -> Self {
        Self::Contract(error)
    }
}

impl From<io::Error> for OllamaAdapterError {
    fn from(_: io::Error) -> Self {
        Self::Unavailable
    }
}

impl fmt::Display for OllamaAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Contract(_) => "Ollama response violated the model contract",
            Self::WrongProvider => "request selected a different provider",
            Self::InvalidEndpoint => "Ollama endpoint is not loopback",
            Self::EncodingFailed => "Ollama request encoding failed",
            Self::Unavailable => "Ollama transport is unavailable",
            Self::TimedOut => "Ollama request timed out",
            Self::Cancelled => "Ollama request was cancelled",
            Self::Disconnected => "Ollama disconnected before completion",
            Self::HeaderTooLarge => "Ollama HTTP headers exceed their bound",
            Self::BodyTooLarge => "Ollama response exceeds its bound",
            Self::MalformedHttp => "Ollama returned malformed HTTP",
            Self::HttpStatus => "Ollama returned a non-success HTTP status",
            Self::UnsupportedHttpEncoding => "Ollama used an unsupported HTTP encoding",
            Self::MalformedResponse => "Ollama returned a malformed response",
            Self::TruncatedResponse => "Ollama response ended before completion",
            Self::EventAfterDone => "Ollama sent data after its terminal response",
        })
    }
}

impl std::error::Error for OllamaAdapterError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_runtime::{
        ConversationMessage, ConversationRole, InferenceRequestId, ModelIntentKind, ModelProfile,
        NormalizedCompletion, NormalizedStreamKind, TurnIntentCatalogue,
    };
    use std::net::{Shutdown, TcpListener};
    use std::thread::{self, JoinHandle};

    fn request(intents: TurnIntentCatalogue) -> InferenceRequest {
        request_with_deadline(intents, 2_000)
    }

    fn request_with_deadline(intents: TurnIntentCatalogue, deadline_ms: u64) -> InferenceRequest {
        InferenceRequest::synthetic(
            InferenceRequestId::parse("ollama-fixture-1".into()).unwrap(),
            ModelProviderKind::Ollama,
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
            assert!(request.len() <= MAX_HTTP_HEADER_BYTES);
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

    fn server(parts: Vec<Vec<u8>>) -> (OllamaAdapter, JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let endpoint = listener.local_addr().unwrap();
        let adapter = OllamaAdapter::for_test(endpoint).unwrap();
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

    fn response(body: &str) -> Vec<Vec<u8>> {
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        vec![header.into_bytes(), body.as_bytes().to_vec()]
    }

    #[test]
    fn fixed_request_and_fragmented_text_stream_normalize_deterministically() {
        let first = r#"{"model":"fixture-model:1","created_at":"2026-09-03T00:00:00Z","message":{"role":"assistant","content":"hel"},"done":false}"#;
        let second = r#"{"model":"fixture-model:1","created_at":"2026-09-03T00:00:01Z","message":{"role":"assistant","content":"lo"},"done":true,"done_reason":"stop","total_duration":1,"load_duration":1,"prompt_eval_count":3,"prompt_eval_duration":1,"eval_count":2,"eval_duration":1}"#;
        let body = format!("{first}\n{second}\n");
        let mut wire = response(&body);
        let body = wire.pop().unwrap();
        let split = body.len() / 2;
        wire.push(body[..split].to_vec());
        wire.push(body[split..].to_vec());
        let (adapter, handle) = server(wire);

        let mut events = Vec::new();
        adapter
            .stream(
                &request(TurnIntentCatalogue::empty()),
                InferenceCancellation::new(),
                |event| events.push(event.clone()),
            )
            .unwrap();
        assert!(matches!(events[0].event, NormalizedStreamKind::Started));
        assert!(matches!(
            &events[1].event,
            NormalizedStreamKind::TextDelta { content } if content == "hel"
        ));
        assert!(matches!(
            &events[2].event,
            NormalizedStreamKind::TextDelta { content } if content == "lo"
        ));
        assert!(matches!(
            events[3].event,
            NormalizedStreamKind::Usage { .. }
        ));
        assert!(matches!(
            &events[4].event,
            NormalizedStreamKind::Finished {
                completion: NormalizedCompletion::Text { content }
            } if content == "hello"
        ));

        let outbound = String::from_utf8(handle.join().unwrap()).unwrap();
        assert!(outbound.starts_with("POST /api/chat HTTP/1.1\r\nHost: 127.0.0.1:"));
        assert!(outbound.contains("\"stream\":true"));
        assert!(outbound.contains("\"num_predict\":512"));
        assert!(!outbound.contains("proxy"));
        assert!(!outbound.contains("https://"));
    }

    #[test]
    fn tool_call_is_only_a_validated_proposal_from_the_turn_catalogue() {
        let body = concat!(
            r#"{"model":"fixture-model:1","created_at":"2026-09-03T00:00:00Z","message":{"role":"assistant","content":"","tool_calls":[{"function":{"name":"process.list","arguments":{}}}]},"done":true,"done_reason":"stop","prompt_eval_count":3,"eval_count":1}"#,
            "\n"
        );
        let (adapter, handle) = server(response(body));
        let catalogue = TurnIntentCatalogue::from_eligible([ModelIntentKind::ProcessList]).unwrap();
        let events = adapter
            .infer(&request(catalogue), InferenceCancellation::new())
            .unwrap();
        assert!(matches!(
            &events[1].event,
            NormalizedStreamKind::ToolIntents { intents }
                if intents.len() == 1 && intents[0].kind() == ModelIntentKind::ProcessList
        ));
        assert!(matches!(
            &events.last().unwrap().event,
            NormalizedStreamKind::Finished {
                completion: NormalizedCompletion::ToolIntents { intents }
            } if intents.len() == 1
        ));
        let outbound = String::from_utf8(handle.join().unwrap()).unwrap();
        assert!(outbound.contains("\"name\":\"process.list\""));
        assert!(outbound.contains("\"additionalProperties\":false"));
        assert!(!outbound.contains(".ssh"));
    }

    #[test]
    fn mixed_unknown_and_unlisted_provider_output_fail_closed() {
        let mixed = concat!(
            r#"{"model":"fixture-model:1","created_at":"2026-09-03T00:00:00Z","message":{"role":"assistant","content":"run it","tool_calls":[{"function":{"name":"process.list","arguments":{}}}]},"done":true,"done_reason":"stop"}"#,
            "\n"
        );
        let (adapter, handle) = server(response(mixed));
        let catalogue = TurnIntentCatalogue::from_eligible([ModelIntentKind::ProcessList]).unwrap();
        assert_eq!(
            adapter.infer(&request(catalogue), InferenceCancellation::new()),
            Err(OllamaAdapterError::Contract(
                ModelContractError::MixedCompletion
            ))
        );
        handle.join().unwrap();

        let unlisted = concat!(
            r#"{"model":"fixture-model:1","created_at":"now","message":{"role":"assistant","content":"","tool_calls":[{"function":{"name":"process.list","arguments":{}}}]},"done":true,"done_reason":"stop"}"#,
            "\n"
        );
        let (adapter, handle) = server(response(unlisted));
        assert_eq!(
            adapter.infer(
                &request(TurnIntentCatalogue::empty()),
                InferenceCancellation::new()
            ),
            Err(OllamaAdapterError::Contract(
                ModelContractError::IntentNotEligible
            ))
        );
        handle.join().unwrap();

        let unknown = concat!(
            r#"{"model":"fixture-model:1","created_at":"now","message":{"role":"assistant","content":"x"},"done":true,"done_reason":"stop","authority":"root"}"#,
            "\n"
        );
        let (adapter, handle) = server(response(unknown));
        let events = adapter
            .infer(
                &request(TurnIntentCatalogue::empty()),
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
    fn redirect_chunk_extensions_and_non_loopback_test_endpoints_are_rejected() {
        let (adapter, handle) = server(vec![
            b"HTTP/1.1 302 Found\r\nLocation: https://example.invalid/\r\nContent-Length: 0\r\n\r\n"
                .to_vec(),
        ]);
        let events = adapter
            .infer(
                &request(TurnIntentCatalogue::empty()),
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

        let (adapter, handle) = server(vec![
            b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\n\r\n1;extension=x\r\nx\r\n0\r\n\r\n"
                .to_vec(),
        ]);
        let events = adapter
            .infer(
                &request(TurnIntentCatalogue::empty()),
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

        assert!(matches!(
            OllamaAdapter::for_test("192.0.2.1:11434".parse().unwrap()),
            Err(OllamaAdapterError::InvalidEndpoint)
        ));
        assert_eq!(
            OllamaAdapter::default().endpoint.to_string(),
            OLLAMA_ENDPOINT
        );
    }

    #[test]
    fn valid_chunked_body_and_invalid_utf8_have_bounded_outcomes() {
        let line = concat!(
            r#"{"model":"fixture-model:1","created_at":"2026-09-03T00:00:00Z","message":{"role":"assistant","content":"chunked"},"done":true,"done_reason":"stop"}"#,
            "\n"
        );
        let split = line.len() / 2;
        let wire = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{}\r\n{:x}\r\n{}\r\n0\r\n\r\n",
            split,
            &line[..split],
            line.len() - split,
            &line[split..]
        );
        let (adapter, handle) = server(vec![wire.into_bytes()]);
        let events = adapter
            .infer(
                &request(TurnIntentCatalogue::empty()),
                InferenceCancellation::new(),
            )
            .unwrap();
        assert!(matches!(
            &events.last().unwrap().event,
            NormalizedStreamKind::Finished {
                completion: NormalizedCompletion::Text { content }
            } if content == "chunked"
        ));
        handle.join().unwrap();

        let mut body = br#"{"model":"fixture-model:1","created_at":"now","message":{"role":"assistant","content":""},"done":true,"done_reason":"stop"}"#.to_vec();
        body.push(0xff);
        body.push(b'\n');
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        let (adapter, handle) = server(vec![header.into_bytes(), body]);
        let events = adapter
            .infer(
                &request(TurnIntentCatalogue::empty()),
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
    fn unterminated_headers_and_declared_oversized_bodies_stop_at_code_owned_bounds() {
        let mut oversized_header = b"HTTP/1.1 200 OK\r\nX-Unterminated: ".to_vec();
        oversized_header.extend(std::iter::repeat_n(b'x', MAX_HTTP_HEADER_BYTES + 1));
        let (adapter, handle) = server(vec![oversized_header]);
        let events = adapter
            .infer(
                &request(TurnIntentCatalogue::empty()),
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

        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nContent-Length: {}\r\n\r\n",
            MAX_HTTP_BODY_BYTES + 1
        );
        let (adapter, handle) = server(vec![header.into_bytes()]);
        let events = adapter
            .infer(
                &request(TurnIntentCatalogue::empty()),
                InferenceCancellation::new(),
            )
            .unwrap();
        assert!(matches!(
            events.last().unwrap().event,
            NormalizedStreamKind::Failed {
                category: ProviderFailureCategory::OutputLimit
            }
        ));
        handle.join().unwrap();
    }

    #[test]
    fn deadline_and_mid_stream_cancellation_are_terminal() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let adapter = OllamaAdapter::for_test(listener.local_addr().unwrap()).unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            thread::sleep(Duration::from_millis(250));
            request
        });
        let events = adapter
            .infer(
                &request_with_deadline(TurnIntentCatalogue::empty(), 40),
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

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let adapter = OllamaAdapter::for_test(listener.local_addr().unwrap()).unwrap();
        let first = concat!(
            r#"{"model":"fixture-model:1","created_at":"now","message":{"role":"assistant","content":"partial"},"done":false}"#,
            "\n"
        );
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            let header = "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nContent-Length: 4096\r\n\r\n";
            stream.write_all(header.as_bytes()).unwrap();
            stream.write_all(first.as_bytes()).unwrap();
            thread::sleep(Duration::from_millis(300));
            request
        });
        let cancellation = InferenceCancellation::new();
        let trigger = cancellation.clone();
        let cancel_thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(40));
            trigger.cancel();
        });
        let events = adapter
            .infer(&request(TurnIntentCatalogue::empty()), cancellation)
            .unwrap();
        assert!(matches!(
            events.last().unwrap().event,
            NormalizedStreamKind::Cancelled
        ));
        assert!(!events.iter().any(|event| matches!(
            event.event,
            NormalizedStreamKind::Finished { .. } | NormalizedStreamKind::ToolIntents { .. }
        )));
        cancel_thread.join().unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn cancellation_before_connection_and_during_headers_is_terminal() {
        let cancellation = InferenceCancellation::new();
        cancellation.cancel();
        let events = OllamaAdapter::default()
            .infer(&request(TurnIntentCatalogue::empty()), cancellation)
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].event, NormalizedStreamKind::Cancelled));

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let adapter = OllamaAdapter::for_test(listener.local_addr().unwrap()).unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            thread::sleep(Duration::from_millis(300));
            request
        });
        let cancellation = InferenceCancellation::new();
        let trigger = cancellation.clone();
        let cancel_thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(30));
            trigger.cancel();
        });
        let events = adapter
            .infer(&request(TurnIntentCatalogue::empty()), cancellation)
            .unwrap();
        assert!(matches!(
            events.last().unwrap().event,
            NormalizedStreamKind::Cancelled
        ));
        cancel_thread.join().unwrap();
        handle.join().unwrap();
    }
}
