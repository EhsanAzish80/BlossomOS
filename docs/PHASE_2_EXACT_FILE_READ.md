# Phase 2 `files.read:content` evidence

Status: implemented and locally verified on 2026-09-02. Target-Linux evidence is
provided by the Quality workflow.

## Contract

- Tool: `files.read.content` with a closed selected-file scope.
- Capability: `files.read:content`, bound to one exact absolute path plus device,
  inode, size, modification time, and change time.
- Policy: explicit `ask`; only interactive `Approve once` reaches content read.
- Source: one descriptor selected with Linux `openat2` and retained across the
  approval prompt. No command, shell, executor, network, or privilege is used.
- Result: at most 64 KiB of valid UTF-8, exact byte count and SHA-256 provenance.

## Containment

Path validation rejects relative, root-only, trailing-separator, repeated,
`.`/`..`, NUL/control, non-UTF-8 CLI, and oversized paths without normalization.
Linux selection resolves beneath an opened root directory with
`RESOLVE_NO_SYMLINKS`, `RESOLVE_NO_MAGICLINKS`, and `RESOLVE_BENEATH`. There is
no ordinary-open fallback. `O_NONBLOCK` permits safe type inspection before
FIFOs and other special files are rejected.

The opened descriptor is private provider state. Approval binds every identity
field. The provider compares identity before and after one bounded read from the
same descriptor; path replacement can therefore neither redirect the read nor
silently substitute content.

## Privacy and audit

The exact path and identity are shown in the human approval preview. Output is
JSON-escaped so file bytes cannot emit terminal control sequences. Audit stores
only path digest, identity numbers, byte count, content digest, transitions, and
verification; it omits clear path and content.

## Tests

Portable tests cover closed request parsing, path forms, identity bounds,
approval binding/replay, approval/denial/cancellation/expiry/non-TTY behavior,
zero executor calls, output escaping, verification, and audit redaction.

Linux-only tests cover the production `openat2` provider, final and intermediate
symlinks, path replacement, FIFO rejection without blocking, oversized files,
invalid UTF-8, mutation after selection, and a real approved CLI flow.

This checkpoint adds no directory browsing, write access, Bash, generic
execution, IPC, LLM, privilege, or graphical shell. The next ADR-0004 item is
one workspace-contained exact-path write, which requires its own accepted design
before implementation.
