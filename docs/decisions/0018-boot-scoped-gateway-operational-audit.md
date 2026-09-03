# ADR-0018: Boot-scoped gateway operational audit

- Status: Accepted
- Date: 2026-09-03
- Accepted: 2026-09-03 as part of the explicitly approved Phase 4 exit work
- Owners: Project maintainers

## Context

ADR-0017 requires reviewable production evidence while prohibiting prompts,
generated text, tool arguments, identities, paths, frames and raw provider
errors from operational records. The existing in-memory model projection is a
testing primitive, not durable gateway evidence, and its full request digest is
derived from prompt content.

## Decision

The gateway owns a fresh, boot-scoped `0700` directory and create-new `0600`
JSON-lines file beneath its systemd runtime directory. Records are bounded,
synced after every append, sequence numbered and SHA-256 chained. Existing,
symlinked, incorrectly owned, incorrectly permissioned, expanded, full,
unwritable or otherwise ambiguous state fails closed.

The record schema contains only lifecycle categories, domain-separated
per-process digests for request IDs and client UIDs, an instance digest, boot
and profile digests, provider/model identifiers in digested or fixed-enum form,
bounded elapsed time, output byte/proposal counts and provider-reported token
counts. It excludes prompts, completions, reasoning, tool arguments/results,
account/group names, raw UID/GID/PID, paths, model bytes, frames, endpoints and
raw errors.

Client admission is recorded before request handling. Request-start evidence
must sync before provider inference begins. A validated terminal frame is held
until terminal evidence syncs; audit failure therefore cannot produce a success
terminal. Protocol rejection and known admission rejection are recorded without
private content.

The journal is operational evidence, not broker audit authority, user memory or
model context. The provider cannot access it. Systemd removes it with the
non-preserved runtime directory at service stop or reboot.

## Consequences

Per-process salts prevent stable cross-restart correlation, but local operators
with access to service state and timing may still correlate activity. A full or
unavailable journal stops new successful inference rather than silently dropping
evidence. Partial text already delivered before a later failure cannot be
recalled; no completed terminal is released without its synced record.

## Validation

Tests must cover chain integrity, exact modes/ownership, stale and expanded
state, bounds, field redaction, domain separation, pre-inference audit failure,
terminal withholding, admission/protocol rejection and installed-service access
denials. Phase 4 remains active until target-Linux installed evidence passes.
