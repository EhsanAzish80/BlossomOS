# Phase 1 Security Core Checkpoint

Status: complete (2026-09-02).

This checkpoint implements the portable deterministic control plane. It does not
claim that a production desktop UI, IPC transport, privileged helper, or LLM
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
- A narrow interactive terminal approval client that displays the request,
  policy decision, once-only scope, exact command, capability, privilege,
  filesystem/network scope, timeout, and output bound before asking.
- Only `Approve once` and `Deny` choices; no permanent or session grant.
- Non-interactive execution denies by default. `Ctrl+C` consumes the pending
  approval as an audited cancellation and starts no executor.
- Approval tokens are internal, are not serializable audit fields, and are never
  printed by the client.
- A readable activity view covering request acceptance, policy, approval,
  execution, verification, and hash-derived audit identifiers.
- Tests for malformed/unknown requests, invalid arguments, default deny, allow,
  ask/approve, user denial, cancellation, expiry, binding, replay,
  non-interactive denial, executor timeout/failure, verification, audit chaining,
  and audit redaction.
- Direct and transitive crate license metadata was reviewed; all resolved
  dependencies use Apache-2.0-, MIT-, Unicode-3.0-, or Unlicense-compatible terms.

## Not implemented yet

- General-purpose execution or caller-selected programs/arguments.
- Writable filesystem scopes, seccomp, Landlock, or cgroup resource limits.
- A graphical approval UI or IPC transport.
- Persistent audit storage or cross-process tamper resistance.
- Any privileged operation or privileged helper.
- Any LLM, planner, memory, Hyprland, or Quickshell code.

## Current trust boundary

Only a typed `ToolRequest` may reach policy. Only an approved request may be
translated into the fixed `CommandSpec`. The executor is an untrusted operational
boundary: its result is recorded and independently verified before success is
reported.

## End-to-end proof

The `blossom-cli` application constructs only `system.uname`; it accepts no
program or argument input. Linux CI executes the approved path through the real
Bubblewrap adapter. Integration tests prove approval, denial, cancellation,
expiration, and non-interactive behavior. The terminal `Ctrl+C` path was also
exercised directly and produced an `ApprovalCancelled` audit event without an
execution event.

## Dependency review

The CLI adds `ctrlc` 3.5.2 solely to turn terminal interruption into a safe,
audited cancellation. A standard-library input thread and channel handle the
prompt; there is no terminal framework. `ctrlc` is dual MIT/Apache-2.0 and its
resolved platform dependencies are license-compatible with Apache-2.0. It adds
no telemetry, network access, dynamic loading, or privilege boundary.

Phase 1 is complete. Phase 2 may add capabilities only through new typed tools,
explicit scopes, containment tests, and any required ADRs; it must not turn this
client into a generic command runner.
