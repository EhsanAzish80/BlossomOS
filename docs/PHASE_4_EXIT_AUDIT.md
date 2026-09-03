# Phase 4 exit audit

Status: Phase 4 remains active. This audit separates implemented repository
controls from evidence that does not yet exist.

## Satisfied repository gates

- ADR-0011 and ADR-0012 are accepted.
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

- No release-constructible closed production profile registry exists.
- No package recipe installs the gateway, accounts, namespace anchor, rendered
  provider unit, manifest, provider binary and model as one reviewed set.
- No production service consumes readiness evidence and the retained
  descriptors at admission time.
- No test has started the packaged services under the intended distinct users
  and verified namespace identity, loopback-only networking, socket ownership,
  peer authorization, filesystem denial and lifecycle failure behavior.
- No pinned real Ollama or llama.cpp runtime and model artifacts are present.
- No real local-model inference has been recorded with external networking
  disabled on the supported target Linux baseline.

## Exit decision

The Phase 4 exit criterion is not satisfied. Controlled-protocol conformance is
not real-model evidence, and static unit analysis is not runtime isolation
evidence. Private and ambient input must remain disabled, the production
gateway must remain fail closed, and Phase 5 must not begin until every item
above is backed by reviewable target-Linux evidence.
