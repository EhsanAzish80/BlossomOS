# Phase 10 beta-candidate release checklist

This checklist assembles evidence; it does not authorize publication.

## Reviewed source

- [x] The candidate revision is the reviewed `main` commit and all required
  branch checks pass.
- [x] The commit satisfies the repository signing policy. Any eventual tag must
  be checked again at publication time.
- [x] No unreviewed runner-local input, credential, model, or package enters the
  candidate.

## Security

- [x] The threat, privilege, and sandbox review matches the candidate source.
- [x] RustSec advisory, dependency review, CodeQL, secret scan, format, lint, and
  test gates pass without an undocumented exception.
- [x] Bounded protocol fuzzing passes and no crash or timeout is discarded.
- [x] Limitations and the disclosure process are current.

## Supply chain

- [x] Release binaries rebuild byte-identically from the same revision and
  `SOURCE_DATE_EPOCH` on the trusted x86-64 Linux runner.
- [x] The bundle contains an SPDX 2.3 SBOM, canonical manifest, and SHA-256
  checksums.
- [x] GitHub signed provenance exists for the candidate bundle and SBOM and
  verifies against `EhsanAzish80/BlossomOS`.
- [x] The workflow artifact digest, provenance identity, commit, and run URL are
  recorded in the exit audit.

## Publication decision

- [ ] Exact prerelease tag, title, notes, artifacts, limitations, and support
  statement are shown to the maintainer.
- [ ] The maintainer explicitly approves publication at that time.
- [ ] The published page, assets, checksums, SBOM, provenance verification, and
  support statement are independently rechecked.

Unchecked publication items do not block Phase 10 beta-candidate evidence, but
they do block creation of a public tag or release.
