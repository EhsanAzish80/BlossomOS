# Phase 4 exit audit

Status: Phase 4 remains active. This audit separates implemented repository
controls from evidence that does not yet exist.

## Satisfied repository gates

- ADR-0011 through ADR-0016 are accepted. ADR-0014 removes guessed numeric
  service IDs from canonical profiles and binds installed IDs at readiness.
- Canonical profiles and readiness bind the complete provider runtime artifact
  set, including bundled dynamic libraries; unknown entries fail closed.
- One x86-64 llama.cpp profile is embedded into release builds from canonical
  bytes, with immutable runtime/model/license pins and a deterministic offline
  package-tree recipe.
- Both provider adapters accept only synthetic crate-owned requests and fixed
  numeric loopback endpoints.
- Request, provider output, proposed intent, gateway frame and normalized event
  schemas are closed, bounded and fail closed before any broker integration.
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
- No production service consumes readiness evidence and the retained
  descriptors at admission time.
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
