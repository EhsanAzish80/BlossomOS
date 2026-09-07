## Purpose

<!-- What problem does this change solve? -->

## Boundary and user-visible behavior

<!-- Describe affected trust boundaries, capabilities, and visible behavior. -->

## Failure and rollback

<!-- Explain denial, cancellation, failure, migration, and rollback behavior. -->

## Verification

- [ ] Relevant deterministic tests pass
- [ ] `python3 scripts/ci/check_repository.py`
- [ ] `cargo fmt --all --check` (when Rust changes)
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` (when Rust changes)
- [ ] `cargo test --all-targets` (when Rust changes)
- [ ] Installed or platform-specific evidence is linked when required

## Documentation and decisions

- [ ] User-facing and evidence documentation is updated
- [ ] An ADR is included for a material architecture or trust-boundary change
- [ ] No completed-state claim exceeds the evidence linked above

## Security and privacy

- [ ] No secret, private content, generated image, model weight, or machine-specific path is committed
- [ ] Model output and repository content are treated as input, not authorization
- [ ] New dependencies follow `docs/DEPENDENCY_POLICY.md`
