# Phase 10 public-beta hardening baseline

Status: active on 2026-09-09. No public beta is declared.

Phase 10 starts from the completed x86-64 UEFI VM boundary in Phase 9. It turns
that evidence artifact into a reviewable beta candidate without expanding the
supported target or silently publishing a release.

## Ordered checkpoints

1. Accept ADR-0027 and freeze the beta-candidate, provenance, publication, and
   support boundary. Complete.
2. Publish a source-backed threat model plus privilege and sandbox review.
3. Enforce Rust dependency updates and a current RustSec advisory gate.
4. Add bounded fuzz targets for untrusted protocol decoders and preserve crash
   evidence without private inputs.
5. Generate and validate a deterministic SPDX 2.3 SBOM, checksums, and canonical
   release manifest from locked inputs.
6. Prove practical reproducibility by rebuilding release binaries from the same
   reviewed commit and comparing their SHA-256 digests.
7. Generate short-lived signed build provenance in the protected candidate
   workflow and verify it against the repository identity.
8. Publish clear limitations, supported-target scope, security-report handling,
   and release/support policy.
9. Run the complete candidate checklist on the trusted x86-64 Linux runner.
10. Perform an independent Phase 10 exit audit and update the roadmap only after
    every gate passes.

## Non-goals

Phase 10 does not claim physical-device support, daily-driver readiness, legacy
BIOS, another architecture, Secure Boot, disk encryption, dual boot, unattended
updates, remote administration, public hardware compatibility, or a stable
release. It does not publish a tag or GitHub prerelease without a separate human
decision at publication time.

## Exit evidence

- Accepted ADR-0027 and completed hardening reviews.
- Protected dependency, fuzz, SBOM, rebuild, and provenance checks.
- One complete beta-candidate evidence bundle tied to a reviewed commit.
- Published limitations, supported-target, disclosure, and release checklist.
- Independent Phase 10 exit audit with links to immutable runs and digests.
