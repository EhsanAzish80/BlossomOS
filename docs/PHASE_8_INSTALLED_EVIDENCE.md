# Phase 8 installed memory evidence

Status: workflow implemented; real installed x86-64 Linux run pending.

The manually dispatched `Phase 8 installed memory evidence` workflow targets
the dedicated self-hosted Linux x86-64 runner. It builds the production Rust
example in a digest-pinned container, then executes the resulting binary
directly on the installed host filesystem.

The probe requires x86-64 and verifies:

- disabled-by-default initialization and explicit enable;
- exact once-only approval for create, edit, and delete;
- encrypted data/key separation and `0700`/`0600` permissions;
- absence of the approved plaintext marker in the encrypted store;
- encrypted persistence across service reconstruction;
- inspect, bounded data-only recall, and explicit JSON export;
- verified delete, content-free hash-chained audit, and disable; and
- cleanup of its isolated temporary evidence directory.

This is lifecycle evidence, not secure-erasure proof for physical media,
hardware-backed key custody, backup/recovery, multi-process coordination,
distribution packaging, upgrade/rollback, or broad hardware support.
