# Phase 2 `process.read:list` evidence

Status: implemented and locally verified on 2026-09-02. Target-Linux evidence is
provided by the Quality workflow.

## Contract

- Tool: `process.list` with an empty, deny-unknown-fields argument object.
- Capability: `process.read:list`, fixed to processes whose effective user ID
  equals Blossom's effective user ID. The caller cannot select a PID or scope.
- Policy: `ask`; only interactive `Approve once` reaches the provider. A denial,
  cancellation, expired approval, or non-TTY invocation starts no read.
- Source: bounded native reads of `/proc/<pid>/status`; no subprocess, shell, or
  executor.
- Result: at most 256 entries containing only PID, short kernel name, and coarse
  state, plus bounded skipped-entry and truncation metadata.

## Containment and race behavior

The reader bounds the proc directory entries, each status record, lines, names,
and returned entries. Numeric PID directories are opened with `O_DIRECTORY`,
`O_NOFOLLOW`, and `O_CLOEXEC`; `status` is then opened relative to that pinned
directory descriptor with `O_NOFOLLOW`, `O_NONBLOCK`, and `O_CLOEXEC`. The PID
inside the single status read must equal the enumerated PID. This rejects proc
substitution and prevents a later PID reuse from redirecting the open operation
to a new process.

Disappearing, inaccessible, special, oversized, or malformed entries become a
typed skipped count. Other-user records are excluded, not skipped. Unknown
valid status fields are ignored rather than added to the public schema.

## Privacy and audit

The implementation does not open `cmdline`, `environ`, `fd`, `maps`, `mem`,
`net`, or any caller-selected proc path. Process names and PIDs are shown only
to the approving local user. Audit records contain the fixed source, returned
count, skipped count, and truncation flag, but no process names or identifiers.

## Verification and tests

Verification checks the source marker, result bound, positive strictly sorted
unique PIDs, and bounded control-free names without reopening procfs. Tests
cover same-user filtering, PID mismatch, malformed and oversized status data,
control characters, missing fields, exact preview, approval, denial,
cancellation, expiry, non-interactive denial, audit redaction, and zero executor
calls. The Linux CI case reads real procfs through the production provider.

This capability does not add generic process targeting, command arguments,
Bash, IPC, an LLM, privilege, write access, or graphical components. The next
ADR-0004 capability is a user-selected exact-file read.
