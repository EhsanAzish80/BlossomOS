# Phase 8 installed memory evidence

Status: complete on 2026-09-07 at merge commit
`a5f94eb44d33ef21bab12b69041dab95a52783fc`.

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

Workflow run
[`34134127972`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34134127972)
passed every step on the dedicated installed Ubuntu x86-64 runner. The runner
validated its installed OS and architecture before building in the
digest-pinned Rust container and executing the resulting probe directly on the
host. The authoritative lifecycle job was
[`101780969151`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34134127972/job/101780969151).

This is lifecycle evidence, not secure-erasure proof for physical media,
hardware-backed key custody, backup/recovery, multi-process coordination,
distribution packaging, upgrade/rollback, or broad hardware support.
