# ADR-0002: Rust for the Phase 1 security core

- Status: Accepted
- Date: 2026-09-02
- Owners: Project maintainers

## Context

Phase 1 implements the capability broker, policy evaluation, approval binding,
executor boundary, verification, and audit records. These components process
hostile structured input and require explicit types, bounded behavior, and small
reviewable modules. Development must work on macOS while Linux-specific execution
remains isolated for later integration testing.

## Decision

Implement the portable Phase 1 security core in stable Rust. Use traits and typed
data structures to isolate Linux-specific execution. Keep dependencies minimal,
commit `Cargo.lock`, deny compiler and Clippy warnings in CI, and require format
and test checks.

This decision does not require the Quickshell UI, privileged helper, or all future
Blossom components to use Rust.

## Alternatives considered

- Python: fast to prototype but provides weaker compile-time guarantees at the
  primary hostile-input boundary and is already associated with the legacy stub.
- Go: simple deployment and concurrency, but Rust offers tighter ownership and
  representation control for this security-sensitive core.
- C++: suitable for Qt integration but introduces unnecessary memory-safety risk
  for the broker and policy core.

## Security and privacy consequences

Rust reduces broad classes of memory-safety defects but does not guarantee safe
authorization logic. All inputs remain untrusted, unsafe Rust is prohibited in
Phase 1 without a separate ADR, and negative tests remain mandatory.

## Operational consequences

CI and contributors require the stable Rust toolchain with `rustfmt` and Clippy.
The core must remain testable without Arch Linux, Hyprland, or root privileges.

## Migration and rollback

Phase 1 starts as a new workspace without modifying the preserved prototype. If
the choice proves unsuitable before a public API exists, supersede this ADR and
replace only the new Phase 1 code.

## Validation

- `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` pass once
  the workspace exists.
- Core tests run on macOS and Linux.
- Linux-specific executor tests remain separately identifiable.
