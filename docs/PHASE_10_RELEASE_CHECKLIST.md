# Phase 10 beta-candidate release checklist

This checklist assembles evidence; it does not authorize publication.

## Reviewed source

- [ ] The candidate revision is the reviewed `main` commit and all required
  branch checks pass.
- [ ] The commit and any eventual tag satisfy the repository signing policy.
- [ ] No unreviewed runner-local input, credential, model, or package enters the
  candidate.

## Security

- [ ] The threat, privilege, and sandbox review matches the candidate source.
- [ ] RustSec advisory, dependency review, CodeQL, secret scan, format, lint, and
  test gates pass without an undocumented exception.
- [ ] Bounded protocol fuzzing passes and no crash or timeout is discarded.
- [ ] Limitations and the disclosure process are current.

## Supply chain

- [ ] Release binaries rebuild byte-identically from the same revision and
  `SOURCE_DATE_EPOCH` on the trusted x86-64 Linux runner.
- [ ] The bundle contains an SPDX 2.3 SBOM, canonical manifest, and SHA-256
  checksums.
- [ ] GitHub signed provenance exists for the candidate bundle and SBOM and
  verifies against `EhsanAzish80/BlossomOS`.
- [ ] The workflow artifact digest, provenance identity, commit, and run URL are
  recorded in the exit audit.

## Publication decision

- [ ] Exact prerelease tag, title, notes, artifacts, limitations, and support
  statement are shown to the maintainer.
- [ ] The maintainer explicitly approves publication at that time.
- [ ] The published page, assets, checksums, SBOM, provenance verification, and
  support statement are independently rechecked.

Unchecked publication items do not block Phase 10 beta-candidate evidence, but
they do block creation of a public tag or release.
