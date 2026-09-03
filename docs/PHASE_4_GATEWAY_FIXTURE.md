# Phase 4 synthetic gateway-fixture checkpoint

Status: implemented on the Phase 4 development branch; this is not the ADR-0012
implementation baseline or the Phase 4 exit baseline.

This checkpoint exercises the accepted gateway protocol across an actual Unix
stream and, on target Linux, across a separate process. It remains explicitly
synthetic-only and accepts an ephemeral caller-selected socket path solely for
test isolation. It is not installed or reachable through a production path.

## Implemented evidence

- `SyntheticGatewayClient::connect_at` opens an ephemeral Unix socket, applies
  bounded I/O timeouts, obtains kernel peer credentials from that exact
  connected descriptor, and validates the expected UID/GID before reading the
  hello or writing any request byte.
- After authentication, the client validates the expected gateway profile and
  both protocol versions, sends only a canonical synthetic request, validates
  every normalized event, requires an exact terminal completion, rejects late
  or partial trailing data, and then requires clean EOF.
- `serve_synthetic_gateway_once` validates client credentials before sending a
  hello, accepts exactly one complete canonical request with no coalesced or
  partial second frame, emits a fixed developer-authored text result through
  `ModelStreamState`, and closes its write side.
- Transport errors map to a small content-free taxonomy. OS error text, prompts,
  completions, credentials, and raw payloads are not returned through errors.
- The fixture has no provider adapter, model, broker, executor, approval,
  privileged helper, audit writer, network access, shell, or lifecycle action.

## Controlled-fixture coverage

Portable tests prove a partial second frame cannot hide behind a valid first
frame. Target-Linux CI additionally:

- launches the Rust test binary as a separate fixture-gateway process;
- connects over a unique ephemeral filesystem Unix socket;
- validates real `SO_PEERCRED` values on both ends;
- completes one canonical synthetic text inference; and
- deliberately supplies the wrong expected gateway UID, then proves the server
  observes EOF with zero request bytes.

These tests authenticate two same-user test processes only to exercise the
kernel and ordering mechanics. They do not claim the distinct production UID,
root-owned socket directory, systemd hardening, or provider isolation required
by ADR-0012.

## Deliberately absent

- the fixed production socket or any persistent listener;
- static service identities, group authorization, packages, or systemd units;
- provider manifests, artifact digests, namespace isolation, Ollama, llama.cpp,
  model files, downloads, or GPU access;
- private/ambient input or a public constructor for it; and
- Phase 5 planning or automatic tool execution.

## Next checkpoint

The closed root-owned provider-profile format and synthetic validator evidence
are now documented in `PHASE_4_PROVIDER_MANIFEST.md`. The next checkpoint is the
package-owned static identities, filesystem locations, namespace anchor, and
hardened systemd unit templates. It must remain unable to receive private input
until the production profile registry and runtime identity evidence are bound to
the actual packaged artifacts.
