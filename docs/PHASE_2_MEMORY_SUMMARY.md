# Phase 2 `system.read:memory.summary` evidence

Status: implemented and locally verified on 2026-09-02. Target-Linux evidence is
provided by the Quality workflow.

## Contract

- Tool: `system.memory.summary` with an empty, deny-unknown-fields argument
  object.
- Capability: `system.read:memory.summary`.
- Policy: explicit default `allow` in the CLI; the policy engine itself remains
  deny by default.
- Source: the fixed `/proc/meminfo` path. Callers cannot supply a path, field,
  command, mount, namespace, or executor setting.
- Result: `MemTotal`, `MemAvailable`, `SwapTotal`, and `SwapFree`, converted from
  the kernel's `kB` unit to integer bytes, plus exact-byte provenance.

## Boundary and privacy

`ProcMeminfoReader` opens the fixed source once with read-only, close-on-exec,
and nonblocking flags. It checks the final opened object is a regular file and
reads at most 64 KiB through that descriptor. It never launches a process.

The parser bounds input and line counts, validates every line's key and numeric
shape, requires each allowlisted field exactly once with the `kB` unit, and uses
checked integer conversion. Unknown valid fields are ignored. Available memory
cannot exceed total memory, and free swap cannot exceed total swap.

The result omits free-memory, cache, slab, active/inactive, huge-page,
commit-limit, hardware, and architecture-specific counters. Audit records only
the fixed source path, byte count, and digest; it does not record memory values.
The values still contribute to device fingerprinting when composed with OS
identity and uptime, so this remains an explicit typed capability rather than a
generic `/proc` reader. `/proc/meminfo` describes the kernel-visible system and
is not claimed to be a cgroup-specific memory limit.

## Verification and tests

Verification checks the fixed path, bounded byte count, lowercase SHA-256 shape,
and value relationships without reopening the source. Tests cover:

- allowlisted parsing, unit conversion, provenance, and unknown-field omission;
- missing or duplicate required fields, invalid units, inconsistent values, and
  overflow;
- malformed keys and values, NUL, invalid UTF-8, line and byte bounds;
- regular symlink targets, missing sources, directories, and bounded reads;
- deny-by-default policy mapping and argument rejection;
- explicit allow through policy, verification, and redacted audit;
- provider failure with no executor fallback;
- CLI rendering with a rejecting executor; and
- on Linux, a real `/proc/meminfo` read with zero executor calls.

This capability does not add Bash, generic execution, caller-selected files,
IPC, an LLM, privilege, write access, or graphical components. The next
capability in ADR-0004 order is `system.read:storage.summary`.
