# Phase 4 gateway-protocol checkpoint

Status: implemented on the Phase 4 development branch; this is not the ADR-0012
implementation baseline or the Phase 4 exit baseline.

This checkpoint implements only ADR-0012's provider-neutral framing and
peer-credential primitives. It remains synthetic-only. It does not create a
gateway process, listener, package, system account, network namespace, provider
profile, or private-input path.

## Implemented evidence

- A binary frame header fixes the eight-byte magic, protocol version, closed
  message kind, zero flags, bounded big-endian payload length, and SHA-256 of
  the exact payload bytes.
- The incremental decoder bounds retained bytes, tolerates arbitrary transport
  fragmentation, rejects empty/oversized frames, unknown kinds, nonzero flags,
  digest mismatch, truncation, and all input after a decoder failure.
- Payloads are canonical, closed JSON schemas. Unknown or duplicate fields,
  noncanonical encodings, invalid UTF-8, and schema expansion fail closed.
- The only request message is explicitly `synthetic_inference`. Decoding
  reconstructs and revalidates the existing provider-neutral request and binds
  its provider kind to the selected CPU evidence profile. There is no private
  classification variant.
- Hello messages bind both protocol versions, one closed profile, a lowercase
  SHA-256 boot correlation, and one bounded process-instance nonce. These are
  correlation fields, not secrets and not substitutes for peer credentials.
- Cancellation carries only one validated request ID. Normalized stream events
  retain the existing schema and add client-side validation of request binding,
  sequence, single start/terminal behavior, usage uniqueness, output bounds,
  mutual exclusion, and exact final-completion equivalence.
- On Linux, `SO_PEERCRED` is read from the connected Unix stream descriptor via
  the safe `nix` API. Pure validation rejects root as the expected gateway UID,
  zero PIDs, and any UID/GID mismatch. Other platforms fail as unsupported.
- No unsafe Rust, new direct dependency, provider call, tool execution, broker
  call, credential token, raw shell, or logging of payload content was added.

Enabling the already-used `nix` crate's socket feature adds its locked
`memoffset` transitive dependency; no new top-level dependency is introduced.

## Controlled-fixture coverage

The fixture suite covers fragmented deterministic framing, digest tampering,
profile mismatch, attempted private-classification expansion, canonical
synthetic request reconstruction, closed cancellation, normalized event round
trips, final-output substitution, post-terminal reuse, cancellation winning at
sequence zero, and peer-credential mismatch. Target-Linux CI additionally reads
real kernel credentials from a connected Unix socket pair.

## Deliberately absent

- `/run/blossom-model-gateway/inference.sock` or any listener;
- gateway or provider binaries and system accounts;
- systemd units, a private network namespace, manifests, model artifacts, or
  runtime artifact validation;
- a client connector that sends prompts;
- private or ambient input;
- real Ollama, llama.cpp, model, offline, CPU, or GPU evidence; and
- Phase 5 planning or automatic tool execution.

## Next checkpoint

The smallest synthetic-only gateway process and client connector are now
implemented separately in `docs/PHASE_4_GATEWAY_FIXTURE.md`. The next checkpoint
defines and validates root-owned provider manifests before adding production
services. Production paths, packages, static identities, private input, and
provider lifecycle remain absent.
