# Phase 10 exit audit

Status: complete on 2026-09-09 for the reviewed x86-64 UEFI VM
beta-candidate evidence boundary. No public release is declared.

## Accepted boundary

Phase 10 hardens the Phase 9 evidence target with source-backed threat review,
current dependency and advisory gates, bounded protocol fuzzing, deterministic
release metadata, practical release-binary reproducibility, and signed supply-
chain records. It does not expand the supported target beyond one x86-64 UEFI
virtual machine and does not authorize a tag, prerelease, installer, support
promise, or physical-device claim.

The candidate was built on GitHub-hosted x86-64 Linux infrastructure using the
Ubuntu 24.04 runner image release `20260907.300`. The runner is build evidence,
not hardware-compatibility evidence.

## Exit evidence

| Gate | Evidence | Result |
| --- | --- | --- |
| Reviewed boundary | ADR-0027 and [PR 156](https://github.com/EhsanAzish80/BlossomOS/pull/156) | Pass |
| Threat, privilege, and sandbox review | `docs/PHASE_10_THREAT_REVIEW.md`; prior security ADRs and their linked tests | Pass |
| Dependency and advisory policy | Dependabot for Cargo and GitHub Actions; locked Cargo inputs; RustSec, dependency review, and repository gates in [PR 161](https://github.com/EhsanAzish80/BlossomOS/pull/161) | Pass |
| Untrusted protocol fuzzing | Bounded libFuzzer job in [run 34346912701](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34346912701) | Pass |
| Static and secret analysis | CodeQL for Rust, Python, and Actions plus Gitleaks in PR 161 | Pass |
| Reproducible release binaries | Two independent release target directories compared byte for byte in [run 34347458354](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34347458354) | Pass |
| Deterministic metadata | Canonical manifest, SHA-256 list, and SPDX 2.3 SBOM generated from the locked revision; consumer-side extraction and checksum verification repeated independently | Pass |
| Signed build provenance | [Attestation 46236074](https://github.com/EhsanAzish80/BlossomOS/attestations/46236074), Rekor index `2768518584`, verified against repository identity | Pass |
| Signed SBOM | [Attestation 46236078](https://github.com/EhsanAzish80/BlossomOS/attestations/46236078), Rekor index `2768518747`, verified with the SPDX 2.3 predicate | Pass |
| Limitations and disclosure | `docs/PHASE_10_LIMITATIONS.md`, `SECURITY.md`, `docs/BRANCH_RELEASE_POLICY.md`, and `docs/PHASE_10_RELEASE_CHECKLIST.md` | Pass |

## Candidate identity

- Reviewed main commit: `0f6ca2cb6f7ce89da46341f29973f0ef7e72dc34`.
- Commit verification: valid GitHub signature.
- Candidate run: `34347458354`; job `102452350926`; event
  `workflow_dispatch` against `main` after the expected push run was not emitted.
- Candidate archive: `blossom-os-beta-candidate-x86_64.tar.gz`.
- Candidate archive SHA-256:
  `7c2f7f009b74b34ae4ac4712e83b4917405c400724488a866fd2e28234cceaf6`.
- Workflow artifact: ID `10102366162`; upload-container digest
  `f13870efd38554966e06943480d12910bfb9434dbc78e3c654aad55e85c8a9e2`.
- Artifact retention: 14 days. The attestations remain repository records after
  the workflow artifact expires.

After download, the archive was extracted outside the build job. Its checksum
file verified all four release binaries and `blossom-os.spdx.json`. The manifest
bound all five files to the reviewed commit and x86-64 architecture. Default
SLSA provenance verification and explicit SPDX 2.3 attestation verification
both succeeded against `EhsanAzish80/BlossomOS`.

## Exit decision

Every Phase 10 beta-candidate gate passes within the accepted evidence boundary.
Phase 10 is complete. The publication section of the release checklist remains
unchecked by design: creating a tag or public GitHub release requires a new,
exact maintainer approval followed by independent verification of the published
assets. Physical-device support, broader hardware compatibility, daily-driver
readiness, and stable-release support remain unproven.
