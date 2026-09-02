# Phase 2 `system.read:storage.summary` evidence

Status: implemented and locally verified on 2026-09-02. Target-Linux evidence is
provided by the Quality workflow.

## Contract

- Tool: `system.storage.summary` with an empty, deny-unknown-fields argument
  object.
- Capability: `system.read:storage.summary` scoped in code to `/`.
- Policy: explicit default `allow` in the CLI; the policy engine itself remains
  deny by default.
- Source: one native `statvfs("/")` snapshot through the safe `nix` wrapper.
- Result: total filesystem bytes and bytes available to an unprivileged user,
  plus the typed `root_statvfs` source and fixed `/` resource path.

## Boundary and privacy

The caller cannot provide a path, mount, filesystem identifier, command,
argument, or executor configuration. The capability does not enumerate mounts,
devices, labels, filesystem types, filenames, inode counts, reserved space, or
other storage topology. It does not read `/proc/mounts` or launch a process.

Capacity conversion multiplies filesystem fragment counts using checked integer
arithmetic. A zero fragment size, available-block count greater than total
blocks, syscall failure, or arithmetic overflow fails closed. Available space
uses `f_bavail`, the blocks available to an unprivileged process; it deliberately
does not report root-reserved `f_bfree` capacity.

Audit records contain the fixed `/` scope and `statvfs` source but omit capacity
values and filesystem identifiers. Capacity can still contribute to device
fingerprinting when composed with existing system reads, which is why this is a
dedicated typed singleton rather than a generic path or mount query.

## Verification and tests

Verification checks the typed source, exact root scope, positive capacity, and
available-to-total relationship without making a second system call. Tests
cover:

- checked fragment-count conversion;
- zero fragment size, inconsistent counts, and integer overflow;
- deny-by-default policy mapping and rejection of caller-supplied paths;
- explicit allow through policy, verification, and redacted audit;
- provider failure with no executor fallback;
- CLI rendering with a rejecting executor; and
- on Linux, a real root `statvfs` snapshot with zero executor calls.

The `nix` `fs` feature is used only as a safe wrapper around the native API; no
new service, daemon, or executable is introduced. This capability does not add
Bash, generic execution, mount enumeration, caller-selected files, IPC, an LLM,
privilege, write access, or graphical components. The next capability in
ADR-0004 order is `process.read:self`.
