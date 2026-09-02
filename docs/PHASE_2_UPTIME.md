# Phase 2 `system.read:uptime` evidence

Status: implemented and locally verified on 2026-09-02. Target-Linux evidence is
provided by the Quality workflow.

## Contract

- Tool: `system.uptime` with an empty, deny-unknown-fields argument object.
- Capability: `system.read:uptime`.
- Policy: explicit default `allow` in the CLI; the policy engine itself remains
  deny by default.
- Source: the fixed `/proc/uptime` path. Callers cannot provide a path, command,
  mount, namespace, or executor setting.
- Result: wall-clock uptime represented as integer seconds and nanoseconds, plus
  the fixed source path, byte count, and SHA-256 of the exact bytes parsed.
- Privacy: the aggregate CPU idle field is parsed and validated because it is
  part of the kernel format, but is not exposed in the result, CLI, or audit.

## Boundary

`ProcUptimeReader` opens the fixed source once with read-only, close-on-exec, and
nonblocking flags. It checks the final opened object is a regular file, reads a
bounded 128 bytes through that same descriptor, and parses the captured bytes.
It does not launch a process or use the Phase 1 executor.

The parser rejects oversized input, NUL bytes, invalid UTF-8, unexpected field
counts, signs, exponent notation, non-digits, empty or overlong fractions,
multiple decimal points, and integer overflow. Decimal values are parsed with
integer arithmetic; floating point is not used.

## Verification and audit

Verification operates on the returned value without reopening `/proc/uptime`.
It checks the fixed source path, bounded byte count, lowercase SHA-256 shape,
and nanosecond range. The hash-chained audit records request, policy, native-read
start, source provenance, and verification. It does not record duration or the
aggregate idle value.

## Test evidence

The Rust suite covers:

- exact duration parsing and exact-byte provenance;
- integer and one-to-nine-digit fractional values;
- malformed values, NUL, invalid UTF-8, oversized input, and overflow;
- regular symlink targets, missing paths, directories, and bounded reads;
- deny-by-default policy mapping and argument rejection;
- explicit allow through policy, verification, and audit;
- provider failure with no executor fallback;
- CLI output and audit redaction with a rejecting executor; and
- on Linux, a real `/proc/uptime` read while proving the executor receives zero
  calls.

This capability does not add Bash, generic executable arguments, IPC, an LLM,
privilege, file-write access, or graphical components. The next capability in
ADR-0004 order is `system.read:memory.summary`.
