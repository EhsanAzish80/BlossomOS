# Phase 2 Checkpoint: OS Identity

Status: implemented and tested (2026-09-02).

This is the first capability checkpoint governed by ADR-0004. It adds one native,
read-only operation and does not expand command execution.

## Capability contract

- Request: `system.os.identity` with no arguments.
- Capability: `system.read:os.identity`.
- Default policy: `allow`.
- Implementation: native Rust file parsing; no subprocess, shell, executor,
  Bubblewrap invocation, network, sudo, or privileged helper.
- Sources: `/etc/os-release`, or `/usr/lib/os-release` only when the former is
  absent. Sources are never merged.
- Public fields: `ID`, `NAME`, `PRETTY_NAME`, `VERSION_ID`,
  `VERSION_CODENAME`, `BUILD_ID`, and `VARIANT_ID` only.

## File and parser boundary

The selected path is opened read-only, nonblocking, and close-on-exec. Symlinks
are followed, including the normal relative `/etc/os-release` symlink. Metadata
and all parsed bytes come through the opened file descriptor; a non-regular final
target is rejected.

The reader limits the file to 64 KiB, 256 lines, 128 assignments, 64 bytes per
key, and 4096 decoded bytes per value. It rejects NUL bytes, invalid UTF-8,
invalid keys, malformed assignments, unsupported concatenated quoting,
unescaped shell-special characters, control characters, excessive input, and
non-regular targets. It parses quoting and the documented escape characters but
never evaluates variables or other shell syntax. Unknown valid fields are parsed
for validity and ignored. Later duplicate assignments win as required by the
os-release specification.

## Verification and audit

The typed result includes the selected logical source path, SHA-256 digest of the
exact bytes parsed, byte count, and allowlisted fields. Verification checks the
typed schema, bounds, source/path agreement, and lowercase SHA-256 shape without
reopening the file.

Audit events record request, `allow` policy decision, native-read start, source
path, byte count, digest, verification, and hash-derived audit IDs. Identity field
values are not copied into audit records.

## Evidence

- Arch and quoted-value fixtures, escaped special characters, Unicode, duplicate
  keys, and unknown-field filtering.
- Missing primary/fallback files and exclusive fallback behavior.
- Bounded file, line, key, and value handling.
- Explicit malformed assignment, NUL, invalid UTF-8, and invalid value errors.
- Relative symlink acceptance, dangling symlink rejection, directory rejection,
  and character-device rejection without blocking.
- Engine tests prove policy, verification, and audit are used while the command
  executor receives zero calls.
- CLI integration tests prove only allowlisted fields are exposed and audit data
  remains redacted.
- Linux CI reads the runner's real os-release through the native path, verifies
  it, and again proves zero executor calls.

## Dependency note

The core declares the already-resolved `libc` crate directly to use the platform
`O_CLOEXEC` and `O_NONBLOCK` constants with Rust's safe `OpenOptionsExt` API. No
unsafe block or new resolved package is introduced. `libc` is dual
MIT/Apache-2.0 and adds no telemetry, networking, dynamic loading, or privilege
boundary.

## Deferred

This checkpoint adds no uptime, memory, storage, process, file-content, write, or
service capability. Persistent audit storage and target-Arch validation remain
future work. The next capability in ADR-0004 order is `system.read:uptime`.
