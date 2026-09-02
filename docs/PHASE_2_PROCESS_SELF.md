# Phase 2 `process.read:self` evidence

Status: implemented and locally verified on 2026-09-02. Target-Linux evidence is
provided by the Quality workflow.

## Contract

- Tool: `process.self` with an empty, deny-unknown-fields argument object.
- Capability: `process.read:self`, intrinsically scoped to the calling Blossom
  process. No PID is accepted from the caller.
- Policy: explicit default `allow` in the CLI; the policy engine itself remains
  deny by default.
- Source: safe native `getpid`, `getppid`, `geteuid`, and `getegid` wrappers.
- Result: current process ID, parent process ID, effective user ID, effective
  group ID, and a typed native-source marker.

## Boundary and privacy

The operation does not open or parse `/proc`, launch a process, or accept a
target. It does not expose process names, executable or working-directory paths,
arguments, environment, credentials beyond numeric effective IDs, threads,
memory, open files, sockets, namespaces, capabilities, or other processes.

Using calling-process APIs eliminates path traversal, procfs symlink behavior,
selection-to-open races, disappearing-process handling, and PID-reuse ambiguity:
there is no externally selected PID and no second process lookup. The parent PID
is a point-in-time native observation and may naturally change if the process is
reparented.

PID, parent PID, user ID, and group ID are returned to the requesting local user
but omitted from audit records. The audit stores only that the fixed self scope
was read through native process-identity APIs. The numeric IDs are still private
system context, which is why the operation remains a dedicated typed capability.

## Verification and tests

Verification checks the typed source and requires a positive current PID without
making another system call. Tests cover:

- valid, invalid, and overflowing identifiers;
- the real current PID through native APIs;
- deny-by-default policy mapping and caller-PID rejection;
- explicit allow through policy, verification, and redacted audit;
- provider failure with no executor fallback;
- CLI rendering with a rejecting executor; and
- on Linux, real native identity calls with zero executor calls.

This capability does not add `/proc` process inspection, a process list, Bash,
generic execution, IPC, an LLM, privilege, write access, or graphical components.
The next capability in ADR-0004 order is `process.read:list`, which requires an
explicit once-only approval and a separate privacy boundary.
