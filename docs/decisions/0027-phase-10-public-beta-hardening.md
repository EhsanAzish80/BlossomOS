# ADR-0027: Phase 10 public-beta hardening boundary

- Status: Accepted
- Date: 2026-09-09
- Owners: Blossom OS maintainers

## Context

Phase 9 proved one installation and update lifecycle on a reviewed x86-64 UEFI
virtual-machine target. It did not establish a supported public release. Phase
10 needs a bounded security and supply-chain gate that cannot turn test evidence
into broad hardware, daily-use, or support claims.

## Decision

The first beta-candidate boundary remains one x86-64 UEFI virtual machine. A
candidate may be assembled only from the reviewed repository and must include:

- a source-backed threat, privilege, and sandbox review;
- locked dependency and advisory checks;
- bounded fuzzing of untrusted protocol decoders;
- an SPDX 2.3 SBOM, SHA-256 checksums, and a canonical release manifest;
- a practical rebuild comparison for the release binaries;
- short-lived, identity-bound build provenance from the protected workflow;
- explicit limitations, supported-target, disclosure, and release-checklist
  documents.

The repository never stores a long-lived release key. A workflow attestation is
not a security guarantee: it binds artifacts to source and build instructions.
A GitHub prerelease, release tag, or support promise remains a separate human
publication decision after all gates pass.

## Alternatives considered

- **Declare Phase 9 artifacts beta-ready.** Rejected because they have no SBOM,
  current advisory gate, fuzz evidence, provenance, or beta support statement.
- **Claim the tested Intel laptop as supported hardware.** Rejected because it
  is runner infrastructure, not a Blossom installation compatibility result.
- **Commit a project signing key.** Rejected because repository compromise would
  compromise every release signed by that key.
- **Require byte-identical full ISO output immediately.** Deferred because the
  current image pipeline contains host and filesystem metadata. Phase 10 first
  requires byte-identical release binaries and deterministic metadata, while
  recording the full image digest and build identity.

## Security and privacy consequences

Every candidate is traceable to reviewed source and a closed dependency graph.
Malformed external protocol inputs receive continuous parser pressure. The
review explicitly accounts for UID separation, filesystem access, network
denial, approval authority, audit minimization, and failure behavior. No new
telemetry, online update, remote administration, or ambient permission is added.

## Operational consequences

Security and beta-candidate workflows become required Phase 10 evidence. They
may take longer than ordinary quality checks and must not run untrusted pull
request code on the self-hosted runner. Publication remains manual and cannot be
triggered by a pull request.

## Migration and rollback

The new checks add no runtime capability and can be removed without changing an
installed system. A failing gate blocks candidate status. Existing Phase 9
evidence remains valid within its narrower boundary.

## Validation

Phase 10 requires the ordered checkpoints in `docs/PHASE_10_BASELINE.md`, a
protected beta-candidate workflow, locally reproducible metadata tests, a real
advisory scan, bounded fuzz runs, signed provenance verification, and an
independent exit audit.
