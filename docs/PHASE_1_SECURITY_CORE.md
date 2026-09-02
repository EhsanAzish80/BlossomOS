# Phase 1 Security Core Checkpoint

Status: in progress.

This checkpoint implements the portable deterministic control plane. It does not
claim that a production sandbox, UI, IPC transport, privileged helper, or LLM
exists.

## Implemented

- Strict JSON request parsing with unknown-field rejection.
- One fixed diagnostic tool, `system.uname`, with no caller-controlled arguments.
- Explicit `system.read:kernel.identity` scoped capability and default-deny policy
  evaluation.
- `allow`, `deny`, and `ask` decisions.
- Time-bounded approval tokens bound to the exact typed request.
- Approval denial, expiration, mutation rejection, and replay rejection.
- A narrow executor trait receiving an argv-style `CommandSpec`, cleared
  environment, fixed working directory, timeout, output limit, and no-network
  intent.
- A Linux Bubblewrap adapter that accepts only the fixed diagnostic, exposes
  `/usr` read-only, creates minimal proc/dev/tmp views, unshares network and other
  namespaces, drops capabilities, disables nested user namespaces, and has no
  unsandboxed fallback.
- Timeout killing and a combined stdout/stderr capture limit outside the sandbox.
- Deterministic result verification.
- SHA-256 hash-chained structured audit records.
- Audit output redaction: command output is represented by byte counts and
  digests, not raw content.
- Tests for malformed/unknown requests, invalid arguments, default deny, allow,
  ask/approve, user denial, expiry, binding, replay, executor timeout/failure,
  verification, audit chaining, and audit redaction.
- Direct and transitive crate license metadata was reviewed; all resolved
  dependencies use Apache-2.0-, MIT-, Unicode-3.0-, or Unlicense-compatible terms.

## Not implemented yet

- General-purpose execution or caller-selected programs/arguments.
- Writable filesystem scopes, seccomp, Landlock, or cgroup resource limits.
- An approval UI or IPC transport.
- Persistent audit storage or cross-process tamper resistance.
- Any privileged operation or privileged helper.
- Any LLM, planner, memory, Hyprland, or Quickshell code.

## Current trust boundary

Only a typed `ToolRequest` may reach policy. Only an approved request may be
translated into the fixed `CommandSpec`. The executor is an untrusted operational
boundary: its result is recorded and independently verified before success is
reported.

The next checkpoint must connect the real adapter to a minimal non-graphical
approval/activity surface and exercise the complete flow without adding a generic
shell path.
