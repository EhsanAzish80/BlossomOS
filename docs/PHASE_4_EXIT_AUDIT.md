# Phase 4 exit audit

Status: Phase 4 remains active. This audit separates implemented repository
controls from evidence that does not yet exist.

## Satisfied repository gates

- ADR-0011 through ADR-0018 are accepted. ADR-0014 removes guessed numeric
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
  profile. Canonical schema v5 also binds the logical model identity.
- The authorized-stream handler enforces one request, rejects pipelining,
  watches cancellation concurrently, and emits only validated events.
- The production listener lifecycle is implemented behind the non-default
  `production-private-inference` package feature. Default builds remain closed.
  The gated path refuses stale socket paths, verifies exact socket metadata,
  authorizes kernel peer credentials before hello/input and retains readiness.
- The feature-gated path creates a bounded, synced, hash-chained boot journal;
  request-start audit failure starts no inference and terminal evidence is
  durable before the terminal frame is released.
- Linux peer-credential primitives and a separate-process synthetic Unix
  gateway are tested.
- Both real adapter implementations are exercised through authenticated
  synthetic gateway framing against deterministic protocol fixtures on Linux.
- Root-owned manifest, account and artifact readiness validation exists and
  retains the validated artifact descriptors.
- Inactive sysusers and hardened systemd templates pass repository drift checks
  and `systemd-analyze verify` in Linux CI.
- Release/default gateway startup exits not-ready before creating a listener.
- Merged commit `7a1b159` has passing disposable x86-64 Linux installed-system
  evidence in workflow run
  [`33852144280`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/33852144280).
  The pinned llama.cpp package passed installation, distinct service identity,
  private namespace and external-network denial, socket admission, real offline
  inference, installed filesystem denials and read-only package mounts, audit
  isolation/redaction, provider-loss non-success, orderly socket cleanup and
  stale-path-refusal checks.

## Unsatisfied production gates

- The production registry/package set is incomplete: the pinned llama.cpp
  x86-64 entry exists, but Ollama and other supported architectures do not.
- The llama.cpp recipe and feature-gated gateway have now been installed and
  exercised on a disposable Ubuntu x86-64 baseline, but no target-Arch package
  or ABI evidence is recorded.
- Production startup consumes readiness evidence and retains its descriptors
  through admission. The listener remains disabled in default packages.
- ADR-0017 fixes the private admission and cancellation contract. Retained
  account-snapshot membership checks and their negative tests are implemented,
  and the isolated one-request/cancellation handler and gated listener are
  implemented. Installed evidence now covers the primary llama.cpp path, but
  not every cancellation and lifecycle edge case.
- Installed llama.cpp evidence covers intended distinct users, namespace
  identity, external-network denial, socket ownership, peer authorization and
  provider-loss behavior, filesystem denials and selected lifecycle cases.
- The installed journal passed ownership, access-denial and redaction checks;
  audit-capacity failure and all terminal-write failure paths remain unproved.
- No pinned real Ollama package or installed Ollama inference evidence exists.
- Cancellation races still lack installed-service evidence.

## Exit decision

The Phase 4 exit criterion is not satisfied. Real-model installed evidence now
exists for the pinned llama.cpp profile, but Ollama packaging/evidence,
target-Arch evidence and the remaining adversarial cases are unresolved.
Private and ambient input must remain disabled in default packages, the default
gateway must remain fail closed, and Phase 5 must not begin until every item
above is backed by reviewable evidence.
