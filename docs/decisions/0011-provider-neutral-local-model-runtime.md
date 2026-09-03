# ADR-0011: Provider-neutral local model runtime boundary

- Status: Proposed
- Date: 2026-09-03
- Owners: Project maintainers

## Context

Phases 1 through 3 established that requests, policy, approval, execution,
verification, privileged operations, and audit remain deterministic and outside
model control. Phase 4 may add local inference, but a model response is
untrusted data rather than permission or executable intent.

Provider replacement must not change Blossom's capability boundary. Ollama and
llama.cpp expose different local HTTP formats for streaming, structured output,
and tool calls. Treating either wire format as Blossom's internal protocol would
couple the agent runtime to that provider and could allow provider-specific
fields to bypass validation.

Both providers also offer features that Blossom must not inherit implicitly.
In particular, a provider may support its own tools, file access, remote models,
downloads, cloud endpoints, or server-side agent loops. Those facilities are
outside the trusted Blossom tool path.

## Decision

### Trust and authority

The model provider is an untrusted inference component. It may generate text or
propose a tool intent, but it cannot:

- call a Blossom tool, broker, executor, privileged helper, or D-Bus method;
- grant, retain, expand, or satisfy an approval;
- construct an internal `ToolRequest`, selected-file identity, retained
  descriptor, idempotency key, or privileged request;
- select filesystem, network, sandbox, process, polkit, or systemd policy;
- report execution or verification success; or
- write directly to the audit journal.

All proposed tool intents re-enter the existing typed registry, policy,
approval, execution, verification, and audit path. Unknown tools, fields, and
arguments fail closed. Prompt text, retrieved content, tool output, model
reasoning, and provider metadata never count as authorization.

### Provider-neutral Rust contract

Add a provider-neutral Rust module before either real adapter. Its closed,
versioned types cover:

- a bounded request identifier and conversation messages;
- the user-selected provider and model profile;
- code-owned tool-intent schemas derived from the registered Blossom tools;
- output mode: user-facing text or a strict Blossom turn schema;
- deterministic generation limits and a total deadline;
- ordered stream events for start, bounded text delta, completed proposed tool
  intent, usage, finish, cancellation, and failure; and
- a cancellation handle that is scoped to exactly one inference request.

Roles are limited to `system`, `user`, `assistant`, and `tool`. Message content,
message count, total serialized request size, tool count, tool-schema size,
tool-call count, argument size, text delta, accumulated output, metadata, and
provider error detail all have code-owned bounds. Unknown enum variants or JSON
fields are rejected at every provider boundary.

The normalized completion is either:

1. bounded assistant text;
2. one or more bounded proposed tool intents; or
3. an explicit cancelled, timed-out, unavailable, malformed, or provider-failed
   outcome.

A provider may stream partial text for display, but partial JSON or partial tool
arguments are buffered privately and are not emitted as a valid intent. A tool
intent becomes visible to the runtime only after the complete provider payload
passes the closed Blossom schema.

### Tool-intent boundary

Provider tool definitions are projections of Blossom's code-owned intent
registry, not provider authority. A proposed call contains only a registered
intent name and its public intent arguments. The runtime assigns correlation and
request identifiers itself.

Resource selection remains outside the model:

- an exact file read still requires the user-selection layer to obtain and pin
  one file identity before approval;
- a workspace write still requires the retained workspace and parent
  descriptors plus the existing create-only constraints;
- service and process scopes remain subject to their typed parsers; and
- privileged operations keep their independent polkit authorization and replay
  journal.

A model-provided pathname is at most untrusted suggestion text. It is never
converted directly into file authority. Provider-native or server-side tool
execution is disabled and unsupported.

### Local transport and privacy

The first adapters connect only to literal loopback addresses and exact paths:

- Ollama: `127.0.0.1:11434`, `POST /api/chat`;
- llama.cpp: `127.0.0.1:8080`, `POST /v1/chat/completions`.

Production requests cannot supply a scheme, host, port, path, redirect target,
proxy, Unix command, or executable. HTTP redirects, environment proxy discovery,
and cleartext non-loopback destinations are disabled. Test-only constructors may
use an ephemeral loopback port for controlled protocol services.

The initial runtime connects to an already-running, user-managed local provider.
It does not spawn a provider, download a binary or model, discover LAN servers,
or fall back to a remote/cloud endpoint. Provider and model profile selection is
an explicit local user setting; model output cannot change it. A later
configurable endpoint or provider lifecycle manager requires a separate threat
review and ADR.

Conversation content and tool results are private local data. Requests are sent
only to the selected local provider. Audit records contain correlation,
provider kind, a bounded model-profile identifier or digest, request/result
digests, timing, token counts when trustworthy, finish category, schema result,
and cancellation state. Prompts, generated text, reasoning text, tool-result
content, credentials, and raw provider errors are omitted by default.

Loopback is not an authentication boundary against another process running as
the same user. Provider responses therefore remain untrusted even after a
successful connection. Initial packaging must bind provider services to
loopback only and should prefer a private authenticated transport when both
supported providers can satisfy one without weakening replaceability.

### Streaming, cancellation, and failure

Stream events carry a monotonically increasing sequence number and exactly one
request identifier. The runtime accepts one terminal event only. Events after a
terminal outcome, duplicate finish events, out-of-order sequence numbers,
oversized deltas, unknown event types, invalid UTF-8, truncated JSON, or tool
calls after cancellation are protocol failures.

Cancellation closes or aborts the in-flight provider request, stops accepting
events, records `cancelled`, and starts no proposed tool. Because a disconnected
HTTP client cannot prove that a provider stopped computing immediately, Blossom
does not claim provider-side termination without provider evidence. Deadline,
disconnect, backpressure failure, and output-limit exhaustion are explicit
non-success outcomes and never trigger an automatic remote retry.

Provider errors are mapped to a small code-owned taxonomy. Raw response bodies
are bounded and redacted before diagnostics. The runtime never interprets an
HTTP success code alone as a valid completion.

### Adapter order

Implementation proceeds in protected checkpoints:

1. provider-neutral types, validator, stream state machine, cancellation, audit
   projection, and scripted conformance suite;
2. Ollama adapter for the fixed local `/api/chat` surface;
3. llama.cpp adapter for the fixed local `/v1/chat/completions` surface;
4. cross-provider conformance and offline target-Linux evidence.

Ollama is first because its documented chat API directly exposes streaming,
JSON-schema output, and tool-call objects. llama.cpp is second because its
OpenAI-compatible server independently exercises the provider abstraction and
supports schema-constrained output and function calling. These upstream
features improve generation shape but never replace Blossom's own validation.

No agent planning loop or automatic tool execution is added in Phase 4. Phase 5
owns orchestration, partial completion, recovery, and truthful user summaries.

### Conformance and exit evidence

Every adapter must pass the same provider-neutral suite using a controlled
loopback protocol service. Evidence includes:

- byte-for-byte request and normalized-event fixtures;
- text, structured output, one and multiple proposed tool intents;
- unknown tool/field/role/finish reason and malformed argument rejection;
- fragmented frames, invalid UTF-8, truncated JSON, duplicate/out-of-order or
  post-terminal events, and oversized input/output;
- timeout, cancellation before connection, during streaming, and after a
  completed intent, proving zero tool executions;
- disconnect, unavailable provider, non-success HTTP status, redirect, proxy,
  and non-loopback rejection;
- provider error redaction and audit omission of prompts and generated content;
- proof that provider payloads cannot select internal request IDs, approvals,
  capabilities, file identities, descriptors, sandbox profiles, privileged
  actions, or transport endpoints; and
- deterministic equivalence of normalized Ollama and llama.cpp fixture results.

Before Phase 4 completes, opt-in target-Linux tests must also run each supported
adapter against a real local provider and small local model with external
network access disabled. Model artifacts and provider binaries must be pinned
and integrity-checked by the test environment; they are not committed to Git.
Passing controlled protocol tests alone is not real-model or offline proof.

Strict Rust formatting, Clippy, tests, dependency review, Gitleaks, CodeQL,
unsafe-code review, and repository policy checks remain required. Any new HTTP
dependency must follow `docs/DEPENDENCY_POLICY.md` and disable default features
that are not used.

## Alternatives considered

### Use one OpenAI-compatible schema internally

Rejected. Compatibility layers differ in streaming, tool-call, error, usage,
and structured-output semantics. Blossom needs a smaller contract whose
security rules do not drift with an external API.

### Link directly to llama.cpp

Deferred. Direct linking can reduce the unauthenticated loopback surface, but it
couples Blossom to a native ABI, build variants, GPU backends, and process
stability. The HTTP adapter is a narrower first replaceability test.

### Let the provider execute tools

Rejected. It bypasses Blossom policy, approval, confinement, verification, and
audit, and some provider tools can access the local filesystem directly.

### Accept arbitrary local or remote endpoints

Rejected. Caller-selected URLs create SSRF, proxy, redirect, privacy, and silent
cloud-fallback risks. External inference may be considered only by a separate
explicit opt-in privacy and transport ADR.

### Give the model the existing internal request schema

Rejected. Internal requests contain authority-bearing selections and runtime
identifiers that must be constructed by trusted code. The model receives a
narrower intent projection.

## Security and privacy consequences

The design keeps inference replaceable and local while preserving the existing
capability boundary. It prevents a provider response from becoming execution by
construction and makes malformed, late, or oversized streams fail closed.

It does not make model output trustworthy, prevent all prompt injection, prove
that a third-party provider binary is benign, authenticate an unauthenticated
loopback provider, or protect data after the same user or root account is
compromised. Supply-chain review, provider sandboxing, signed packages, model
provenance, and hardware sizing remain separate work.

## Migration and rollback

Phase 4 introduces no released protocol. Each adapter is independently
removable behind the provider-neutral trait. Rollback removes its registration,
transport mapping, fixtures, and optional packaging without changing the broker,
policy, executor, privileged helper, or other provider.

Changing a provider wire API requires adapter fixtures and conformance review;
changing the provider-neutral contract requires a new version or superseding
ADR. No adapter may silently fall back to another provider or endpoint.

## References

- Ollama chat API: <https://docs.ollama.com/api/chat>
- Ollama structured outputs: <https://docs.ollama.com/capabilities/structured-outputs>
- Ollama tool calling: <https://docs.ollama.com/capabilities/tool-calling>
- llama.cpp server: <https://github.com/ggml-org/llama.cpp/tree/master/tools/server>
- llama.cpp grammar and JSON Schema support:
  <https://github.com/ggml-org/llama.cpp/tree/master/grammars>
