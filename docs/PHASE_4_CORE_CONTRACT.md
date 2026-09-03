# Phase 4 core-contract checkpoint

Status: implemented on the Phase 4 development branch; this is not the Phase 4
exit baseline.

This checkpoint implements only the first protected step accepted by ADR-0011.
It adds a provider-neutral, authority-free inference contract and synthetic
conformance fixtures. It does not connect to, launch, download, or manage a
model or provider.

## Implemented evidence

- `core/blossom-core/src/model_runtime.rs` defines closed provider, request,
  conversation, output, intent, stream, failure, cancellation, and audit types.
- Requests and outputs have explicit byte, count, deadline, sequence, and
  terminal-state bounds.
- Request construction is crate-internal and synthetic-only. There is no public
  private-input constructor.
- The default per-turn intent catalogue is empty. Trusted core code can build a
  bounded catalogue containing only currently typed, authority-free intents.
- Code-owned intent definitions accept an empty argument object. Provider
  attempts to supply a path, `~/.ssh`, executable, argument, or unknown intent
  fail validation.
- Text and tool-intent output are mutually exclusive. Unknown fields and mixed
  output fail closed.
- A stream accepts one start, monotonically ordered events, and one terminal
  outcome. A protocol violation poisons the stream; cancellation releases no
  completed intent.
- Audit projections contain correlation data, provider kind, digests, bounded
  counts, and optional token counts. They omit prompts, generated text, raw
  errors, and the model-profile string.
- Byte-stable request, event, and intent-schema fixtures exercise the normalized
  contract independently of a provider transport.

## Deliberately absent

- Ollama and llama.cpp transports;
- model downloads, lifecycle management, or provider discovery;
- private or ambient user input;
- provider endpoint identity or authentication;
- provider-native tool execution;
- capability execution, approvals, planning, Bash, sudo, or shell integration;
  and
- real-model or offline target-Linux evidence.

## Verification

The repository requires formatting, Clippy, workspace tests, repository-policy
checks, Gitleaks, CodeQL, and the existing target-Linux workflows before this
checkpoint may merge. Those checks establish the core contract only; they do
not satisfy the Phase 4 exit criteria.

## Next checkpoint

Implement the fixed-loopback Ollama development adapter for synthetic
conformance prompts only. It must not receive private data, execute proposed
tools, follow redirects, use environment proxies, or choose a caller-supplied
endpoint. Phase 4 remains active until every exit criterion in `ROADMAP.md` is
satisfied.
