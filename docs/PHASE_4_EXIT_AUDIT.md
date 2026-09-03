# Phase 4 exit audit

Status: Phase 4 remains active. This audit separates implemented repository
controls from evidence that does not yet exist.

## Satisfied repository gates

- ADR-0011 through ADR-0017 are accepted. ADR-0014 removes guessed numeric
  service IDs from canonical profiles and binds installed IDs at readiness.
- Canonical profiles and readiness bind the complete provider runtime artifact
  set, including bundled dynamic libraries; unknown entries fail closed.
- One x86-64 llama.cpp profile is embedded into release builds from canonical
  bytes, with immutable runtime/model/license pins and a deterministic offline
  package-tree recipe.
- The package recipe renders a profile-specific gateway unit whose exact
  read-only binds make the selected manifest, runtime set and model visible to
  readiness without exposing the wider Blossom package tree.
- Both provider adapters accept only synthetic crate-owned requests and fixed
  numeric loopback endpoints.
- Request, provider output, proposed intent, gateway frame and normalized event
  schemas are closed, bounded and fail closed before any broker integration.
- The private-inference frame omits provider, model, endpoint, path and
  classification authority; its decoder injects those values from the admitted
  profile. The production listener remains disabled.
- The authorized-stream handler enforces one request, rejects pipelining,
  watches cancellation concurrently, and emits only validated events. It is
  tested through Unix socket pairs but is not yet reachable from production.
- Linux peer-credential primitives and a separate-process synthetic Unix
  gateway are tested.
- Both real adapter implementations are exercised through authenticated
  synthetic gateway framing against deterministic protocol fixtures on Linux.
- Root-owned manifest, account and artifact readiness validation exists and
  retains the validated artifact descriptors.
- Inactive sysusers and hardened systemd templates pass repository drift checks
  and `systemd-analyze verify` in Linux CI.
- Release/default gateway startup exits not-ready before creating a listener.

## Unsatisfied production gates

- The production registry/package set is incomplete: the pinned llama.cpp
  x86-64 entry exists, but Ollama and other supported architectures do not.
- The llama.cpp recipe produces one reviewed package root, but it has not been
  installed or exercised on the supported target Linux baseline.
- Production startup consumes readiness evidence and retains its descriptors
  through the admission decision, but no packaged service has exercised that
  path and the listener remains deliberately disabled.
- ADR-0017 fixes the private admission and cancellation contract. Retained
  account-snapshot membership checks and their negative tests are implemented,
  and the isolated one-request/cancellation handler is implemented, but the
  production listener and installed-service path are not yet implemented.
- No test has started the packaged services under the intended distinct users
  and verified namespace identity, loopback-only networking, socket ownership,
  peer authorization, filesystem denial and lifecycle failure behavior.
- No pinned real Ollama package exists; pinned llama.cpp/Qwen inputs have not
  yet produced target-Linux runtime evidence.
- No real local-model inference has been recorded with external networking
  disabled on the supported target Linux baseline.

## Exit decision

The Phase 4 exit criterion is not satisfied. Controlled-protocol conformance is
not real-model evidence, and static unit analysis is not runtime isolation
evidence. Private and ambient input must remain disabled, the production
gateway must remain fail closed, and Phase 5 must not begin until every item
above is backed by reviewable target-Linux evidence.
