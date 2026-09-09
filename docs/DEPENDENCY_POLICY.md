# Dependency Policy

Blossom OS minimizes dependencies at security and privilege boundaries. A new
dependency must solve a concrete problem better than a small, reviewable local
implementation.

## Requirements

- Record direct runtime and build dependencies in a lockable manifest.
- Commit lockfiles for applications and system services.
- Pin GitHub Actions to immutable commit SHAs with a version comment.
- Use official distribution packages or upstream sources; do not execute
  unaudited remote install scripts.
- Review license compatibility, maintenance activity, release provenance,
  transitive dependencies, platform support, and security history.
- Keep privileged-helper and policy-engine dependency sets especially small.
- No dependency may add telemetry, network access, dynamic code loading, or a new
  privilege boundary without explicit documentation and, when architectural, an
  ADR.
- Model weights and generated binaries are release artifacts, never Git source.

## Updates and vulnerabilities

- Dependabot monitors Cargo and GitHub Actions weekly.
- The protected security workflow scans `Cargo.lock` against the current RustSec
  advisory database and rejects vulnerable, yanked, unmaintained, or unsound
  dependencies. Any temporary exception must name the advisory, document
  reachability and mitigation, have an owner and expiry, and receive focused
  security review.
- Security updates are prioritized and tested before merging.
- Breaking or security-sensitive upgrades receive focused review.
- Abandoned or compromised dependencies must be replaced or isolated.

## Evidence

Pull requests adding a dependency must explain why it is needed, alternatives
considered, license, trust impact, and validation performed.
