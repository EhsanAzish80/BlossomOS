# Branch, Review, and Release Policy

## Branches and review

- `main` is the default integration branch and must remain buildable.
- Normal work uses focused branches and pull requests.
- Security-sensitive changes require focused review separate from unrelated UI or
  refactoring changes.
- Shared history is not rewritten. Force-pushing `main` is prohibited.
- Required checks are Quality, CodeQL, and Secret scan once the workflows are
  present on the default branch.
- The repository owner must configure a GitHub ruleset or branch protection for
  `main` requiring pull requests and required checks. Until enabled, this policy
  is documented but not technically enforced.

## Versions and releases

- Blossom is pre-alpha and has no supported release.
- Development versions use semantic versioning once the first runnable vertical
  slice exists. Versions below `1.0.0` may change incompatibly but must document
  migrations.
- A Git tag is not a supported release unless accompanied by release notes,
  checksums, provenance, known limitations, and an explicit support statement.
- Stable `1.0.0` requires owner approval and completion of the public-beta
  hardening gate in `ROADMAP.md`.

## Signing

- The historical preservation tag remains unchanged.
- Future public release tags must be signed by a documented maintainer key.
- Release artifacts must publish SHA-256 checksums. Artifact signing and
  provenance tooling will be selected before the first distributable image.
- Signing keys, recovery material, and tokens are never stored in this repository.
