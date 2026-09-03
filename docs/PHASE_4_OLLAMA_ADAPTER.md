# Phase 4 Ollama development-adapter checkpoint

Status: implemented on the Phase 4 development branch; this is not the Phase 4
exit baseline.

This checkpoint implements ADR-0011's first provider adapter. It is deliberately
limited to synthetic conformance and developer-authored test prompts. Loopback
is not treated as provider authentication, so private or ambient user data
remains blocked.

## Implemented evidence

- `OllamaAdapter::default()` connects only to the code-owned numeric endpoint
  `127.0.0.1:11434` and sends only `POST /api/chat`.
- The production API accepts no URL, host, port, path, proxy, redirect target,
  command, executable, or provider-launch configuration. An ephemeral loopback
  endpoint constructor exists only in unit-test builds.
- The adapter uses `std::net::TcpStream`; it performs no DNS lookup, proxy
  discovery, redirect following, TLS connection, subprocess launch, binary or
  model download, LAN discovery, or cloud fallback. No dependency was added.
- Requests are available only through the crate-internal synthetic constructor.
  They use bounded messages, deterministic generation settings, `think: false`,
  `logprobs: false`, and the smallest per-turn code-owned tool catalogue.
- Response headers, body bytes, chunks, lines, text deltas, tool calls, fields,
  metadata, sequence, and deadline are bounded and validated before a terminal
  completion is released.
- Assistant text and proposed tool intents remain mutually exclusive. Tool
  calls are converted into authority-free proposals and validated by the
  provider-neutral core; the adapter has no broker or executor reference.
- Cancellation is checked before connecting, while writing, while reading
  headers, and while reading response bodies. It closes the connection and
  emits a terminal cancellation without a completed tool intent.
- Provider and protocol failures map to the small normalized failure taxonomy.
  Raw provider bodies and error details are not returned or audited.
- The public streaming entry point emits normalized events as bounded provider
  frames arrive; partial JSON and partial tool arguments are never emitted as
  valid intents.

## Controlled-protocol coverage

The local fixture suite covers:

- a byte-inspected fixed request and fragmented text response;
- valid content-length and chunked response framing;
- one allowlisted tool proposal with a code-owned empty argument schema;
- mixed text/tool, unlisted tool, unknown field, invalid UTF-8, malformed
  framing, chunk extension, redirect/non-success status, oversized declared
  body, and unterminated oversized header rejection;
- cancellation before connection, during headers, and during a partial stream;
- total deadline enforcement; and
- rejection of a non-loopback test endpoint.

GitHub's target-Linux repository job must pass these fixtures before merge.
That is controlled-protocol evidence only, not real-model evidence and not
provider identity proof.

## Deliberately absent

- private conversation, file, clipboard, notification, desktop, or tool-result
  input;
- authenticated provider identity or Blossom-managed provider packaging;
- provider-native or Blossom tool execution;
- model installation, selection UI, lifecycle management, or real-model tests;
- llama.cpp support in this adapter; and
- planning, Bash, sudo, or graphical-shell integration.

## Next checkpoint

The fixed-loopback llama.cpp development adapter is now implemented separately
in `docs/PHASE_4_LLAMA_CPP_ADAPTER.md`. The next checkpoint is the provider
endpoint-identity and packaging boundary. Phase 4 remains active until every
exit criterion in `ROADMAP.md` is satisfied.
