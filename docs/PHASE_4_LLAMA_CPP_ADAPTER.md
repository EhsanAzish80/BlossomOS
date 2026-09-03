# Phase 4 llama.cpp development-adapter checkpoint

Status: implemented on the Phase 4 development branch; this is not the Phase 4
exit baseline.

This checkpoint implements ADR-0011's second provider adapter. It proves that
the provider-neutral model contract can accept a second, materially different
local streaming protocol. It remains limited to synthetic conformance and
developer-authored test prompts. Loopback is not provider authentication, so
private or ambient user data remains blocked.

## Implemented evidence

- `LlamaCppAdapter::default()` connects only to the code-owned numeric endpoint
  `127.0.0.1:8080` and sends only `POST /v1/chat/completions`.
- The production API accepts no URL, host, port, path, proxy, redirect target,
  command, executable, provider-launch, model-download, or server-management
  configuration. An ephemeral loopback endpoint exists only in unit-test builds.
- Requests are available only through the crate-internal synthetic constructor.
  They use bounded messages, deterministic generation settings, disabled model
  reasoning, disabled parallel tool calls, and the smallest code-owned tool
  catalogue for the turn.
- The adapter uses `std::net::TcpStream`; it performs no DNS lookup, proxy
  discovery, redirect following, TLS connection, subprocess launch, LAN
  discovery, or cloud fallback. No dependency was added.
- The Server-Sent Events decoder accepts only bounded `data:` events and the
  `[DONE]` terminator. It rejects malformed framing, invalid UTF-8, data after
  completion, identity changes, unknown response fields, reasoning content,
  refusals, log probabilities, invalid usage totals, and inconsistent finish
  reasons.
- Fragmented tool names and argument JSON remain internal until the terminal
  response. They are then converted to authority-free proposals and validated
  by the same provider-neutral, code-owned schema used by the Ollama adapter.
- Assistant text and tool proposals remain mutually exclusive. The adapter has
  no broker, policy, approval, executor, privileged-helper, or audit authority.
- Response headers, body bytes, events, metadata, deltas, tool fields, sequence,
  deadline, and output are bounded. Cancellation is checked throughout the
  connection and decoding path.
- Provider failures are mapped to the closed normalized failure taxonomy; raw
  provider bodies and diagnostic detail are not returned or audited.

## Controlled-protocol coverage

The local fixture suite covers:

- byte-inspected fixed request construction and fragmented SSE text;
- fragmented tool-call arguments becoming one validated proposal;
- mixed text/tool output, unlisted tools, reasoning output, unknown fields,
  malformed completion, and inconsistent stream identity failing closed;
- cancellation, deadline, redirect/non-success response, and non-loopback
  endpoint handling; and
- normalized terminal success and failure events.

GitHub's target-Linux repository job must pass these fixtures before merge.
That is controlled-protocol evidence only, not real-model evidence and not
provider identity proof.

## Deliberately absent

- private conversation, file, clipboard, notification, desktop, or tool-result
  input;
- authenticated provider identity or Blossom-managed provider packaging;
- provider-native or Blossom tool execution;
- model installation, selection UI, lifecycle management, or real-model tests;
- arbitrary server agents, media ingestion, remote downloads, router control,
  or cloud fallback; and
- planning, Bash, sudo, or graphical-shell integration.

## Next checkpoint

Define and accept the provider endpoint-identity and packaging boundary before
either adapter receives private or ambient user data. Then add cross-provider
fixture equivalence and real local-model, offline target-Linux evidence. Phase 4
remains active until every exit criterion in `ROADMAP.md` is satisfied.
