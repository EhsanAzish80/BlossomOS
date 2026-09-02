# ADR-0010: Try-restart the fixed Bluetooth service as the first privileged operation

- Status: Accepted
- Date: 2026-09-02
- Owners: Project maintainers
- Requires: ADR-0009

## Context

ADR-0009 defines the system D-Bus and polkit boundary for a future minimal
privileged helper but intentionally exports no method. Phase 3 needs one useful,
low-complexity operation that proves the complete boundary without becoming a
generic root command path.

Restarting a stuck Bluetooth service is a plausible desktop recovery action. It
is disruptive—active devices and audio may disconnect—so it must never be an
implicit model action. The authority can be kept finite by fixing the only unit,
systemd method, job mode, policy action, deadline, observed properties, and
success criteria in code.

systemd's `RestartUnit` also starts an inactive service. The narrower
`TryRestartUnit` does nothing when the unit is not running, so it avoids silently
adding service-start authority. systemd exposes the queued job and a `JobRemoved`
result, while the unit's active state and invocation identity allow Blossom to
verify that a new service invocation actually became active.

## Decision

### User-visible operation

The first privileged operation is:

- tool: `services.bluetooth.try_restart`;
- Blossom capability: `services.restart:bluetooth.service`;
- helper method: `TryRestartBluetooth1`;
- fixed unit: `bluetooth.service`;
- fixed systemd manager method: `TryRestartUnit`;
- fixed job mode: `replace`; and
- fixed polkit action:
  `org.blossomos.privileged1.try-restart-bluetooth`.

The request contains only a version, bounded correlation ID, 128-bit idempotency
key, and an explicit interactive flag. It contains no unit, executable, argument,
path, bus destination, object path, interface, method, job mode, timeout,
environment, credential, polkit action, or sandbox field. Unknown fields and
versions are rejected before authorization.

The result is one of a closed set:

- `restarted_active` with bounded verified post-state metadata;
- `not_running`, with no restart job submitted;
- `unit_unavailable`;
- `denied`, `cancelled`, or `expired`, with no restart job submitted;
- `job_failed` with a bounded allowlisted systemd job-result category;
- `verification_failed`; or
- `outcome_indeterminate` when a job may have started but its terminal state
  cannot be proven.

There is no automatic retry. A later user request receives a new preview,
Blossom approval, polkit authorization, and idempotency key.

### Exact preview and approvals

Before the helper is contacted, the unprivileged client displays:

- exact unit `bluetooth.service`;
- exact capability and fixed `TryRestartUnit(..., "replace")` operation;
- that only an already running service is affected;
- expected Bluetooth device/audio disconnection and reconnection;
- no shell, subprocess, network, file write, package action, enable/disable,
  start-if-inactive, stop-only, signal, unit-file change, or arbitrary service;
- a fixed complete-operation deadline; and
- `Approve once` or `Deny` only.

Non-TTY use, denial, cancellation, or expired Blossom approval does not contact
the privileged helper. The approval binds the complete typed request including
correlation and idempotency identifiers. The token is consumed once and is never
sent to, accepted by, or printed by the helper.

After the once-only Blossom approval, the helper independently performs the
ADR-0009 polkit check. Its policy defaults are `no` for any/inactive subjects and
`auth_admin` for an active local subject, without retained authorization.
Authentication dismissal or challenge failure is denial, not execution.

### Fixed systemd operation

The helper uses native D-Bus only. It never launches `systemctl`, `busctl`,
`pkcheck`, `pkexec`, a shell, or any other process.

After authorization and durable idempotency claim it:

1. connects only to the fixed local system bus;
2. calls `GetUnit("bluetooth.service")`, never `LoadUnit`;
3. reads only `Id`, `LoadState`, `ActiveState`, and `InvocationID` from the fixed
   generic unit interface;
4. validates canonical ID `bluetooth.service`, load state `loaded`, bounded
   state tokens, and a 16-byte non-zero invocation ID;
5. returns `not_running` without mutation unless `ActiveState` is exactly
   `active`;
6. subscribes to the fixed manager `JobRemoved` signal before submitting work;
7. calls only `TryRestartUnit("bluetooth.service", "replace")`;
8. accepts only the returned bounded job object path and waits only for the
   matching job removal under one fixed deadline;
9. requires the terminal job result `done`; and
10. re-reads the same four unit properties and requires `loaded`, `active`, the
    exact canonical ID, and an invocation ID different from the pre-state.

The helper does not call `RestartUnit`, `StartUnit`, `StopUnit`, `ReloadUnit`,
`KillUnit`, `ResetFailedUnit`, `EnableUnitFiles`, `DisableUnitFiles`, `MaskUnitFiles`,
`SetUnitProperties`, any list method, or any caller-selected D-Bus operation. It
does not inspect Bluetooth logs, devices, connections, configuration, process
details, or files.

### Deadlines, cancellation, and outcome truth

The complete post-authorization systemd operation has a code-owned 20-second
deadline, including pre-state, signal subscription, job submission, job wait,
and post-state verification. The polkit interaction has a separate bounded
authorization lifetime suitable for human authentication and is cancellable on
caller disconnect.

Cancellation before `TryRestartUnit` starts no job. Once systemd accepts the
job, Blossom does not cancel it: cancelling a queued job cannot reliably undo
restart work that has begun. Caller disconnect or deadline after submission
therefore yields `outcome_indeterminate` unless the helper can still observe and
record a definitive terminal result. It never reports cancellation or failure
as proof that Bluetooth was not restarted.

`restarted_active` means only that systemd reported the matching restart job
done and a new invocation was observed active at verification time. It is not a
promise that adapters, devices, audio profiles, or connections recovered, and
the service state may change immediately afterward.

### Idempotency and replay

ADR-0009's root-owned boot journal is mandatory. The normalized digest includes
the interface version, method, fixed unit, fixed job mode, authenticated UID,
correlation ID, and idempotency key. The claim is synced before
`TryRestartUnit`.

A duplicate exact request returns the recorded terminal result or current
in-progress/indeterminate state without submitting a second job. Reusing a key
with any different normalized value is denied and audited. Helper restart reloads
the journal. Journal corruption, capacity exhaustion, sync failure, or ambiguous
state fails closed before job submission.

### Audit and privacy

The broker and helper correlate these transitions: request, policy, Blossom
approval or denial, caller identity validation, polkit decision, idempotency
claim, pre-state category, job submission, job result, verification, and final
outcome.

Persistent audit stores the fixed unit/capability/action, correlation ID,
authenticated UID or a stable local digest according to the future audit
retention policy, request digest, idempotency category, coarse pre/post active
category, job-result category, verification result, and audit IDs. It does not
store authentication material, approval tokens, D-Bus credentials beyond the
minimum caller identity, device identifiers, connection names, Bluetooth logs,
arbitrary systemd strings, or process data.

### Helper packaging and hardening

The first implementation introduces a dedicated `system/privileged-helper`
crate only with the first tested component. Packaging includes:

- a root-owned executable at a fixed distribution path;
- a system D-Bus activation file for `org.blossomos.Privileged1`;
- a bus policy that permits root to own the name and permits local callers only
  to send to the one service interface;
- a polkit action file declaring only the fixed Bluetooth action and ADR-0009
  defaults; and
- a systemd `Type=dbus` unit with fixed `BusName`, root-owned runtime journal,
  empty Linux capability sets where target validation permits, and the strictest
  compatible filesystem, device, namespace, address-family, syscall, task,
  memory, and executable-memory restrictions.

The package installs no polkit `.rules` file and no setuid binary. The helper has
no writable host path except its root-owned runtime journal. It needs only local
AF_UNIX D-Bus access and no IP networking.

Exact systemd directives and resource values must be committed with the
implementation, checked by `systemd-analyze verify` for the supported Arch
systemd baseline, and documented rather than weakened until tests pass.

## Alternatives considered

- A generic exact-unit restart argument: rejected because a future caller could
  target security, network, login, storage, or update services.
- `RestartUnit`: rejected because it also starts an inactive service and expands
  authority beyond recovery of a currently running service.
- Call `systemctl try-restart bluetooth.service`: rejected because it adds
  subprocess, executable, argument, environment, and output-parsing surfaces.
- Let the unprivileged broker call systemd directly: useful systemd policy may
  vary by distribution, but it would not prove Blossom's independent privileged
  operation, idempotency, double-audit, and helper hardening boundary.
- Restart a Blossom-owned daemon: rejected for the first operation because no
  production Blossom system daemon exists yet.
- Change timezone or hostname: rejected for the first operation because those
  are persistent configuration changes with input validation, rollback, and
  filesystem semantics broader than one fixed service job.
- Reboot, package installation, user management, firewall, storage, or update
  operations: rejected as too disruptive and compositionally powerful for the
  first privileged slice.
- Treat job submission as success: rejected because it would violate Blossom's
  verification contract.
- Automatically retry timeout or disconnect: rejected because a restart may
  already have occurred.

## Security and privacy consequences

Even this fixed operation can interrupt keyboards, mice, controllers, audio,
and active transfers. The two independent approval steps and exact preview make
that side effect visible but do not remove it. A malicious same-user process can
attempt to trigger an authentication prompt; ADR-0009 rate limits and serializes
prompts, and no mutation occurs without authorization.

The helper runs as root and can ask systemd to restart exactly one unit. A defect
in request dispatch, D-Bus proxy construction, polkit subject binding, journal,
or packaging could be security-critical. Closed enums, fixed constants,
negative-surface tests, empty command execution surface, and systemd hardening
reduce but do not eliminate that risk. Blossom remains pre-alpha.

The operation reveals only coarse Bluetooth service availability and transition
state already within the approved scope. It does not expose device or connection
metadata.

## Implementation and validation gate

The implementation must be reviewed as one separate protected checkpoint and
must include:

- shared versioned request/result types with byte-for-byte schema fixtures;
- exhaustive method-to-capability/action mapping and unknown-method rejection;
- an interactive CLI path only, with exact preview and no helper contact before
  once-only approval;
- helper-side system-bus sender capture and polkit `system-bus-name` checks;
- boot journal persistence, duplicate/no-double-execution, digest mismatch,
  helper-restart, corruption, full-journal, and sync-failure tests;
- controlled private D-Bus services implementing only the required polkit and
  systemd surfaces;
- authorization denial, challenge, dismissal, timeout, caller-disconnect,
  inactive unit, unavailable unit, malformed state, job failure, job timeout,
  wrong-job signal, changed-result, and post-verification-failure tests;
- proof that denial/cancellation/expiry/non-TTY paths submit zero systemd jobs;
- proof that payload data cannot select a unit, method, action, bus address,
  executable, path, or job mode;
- systemd unit, bus policy, activation, polkit XML, file ownership/mode, and
  package-layout validation;
- strict Rust lint/test, repository policy, Gitleaks, CodeQL, dependency review,
  unsafe-code review, and target-Linux evidence; and
- an independent Phase 3 exit review proving no generic root command path.

CI must never restart the host runner's real Bluetooth service. Real hardware or
target-Arch evidence is a later explicit test gate and cannot be inferred from a
controlled D-Bus integration test.

No LLM, Bash, sudo, generic systemd tool, permanent approval, graphical shell,
package manager, or second privileged method is included in this checkpoint.

## Migration and rollback

There is no released helper protocol. Before release, rollback removes the
method, action, bus policy allowance, typed capability, client preview, audit
renderer, package files, and tests together. A helper without a recognized
method remains inert and rejects all calls.

Changing the fixed unit, `TryRestartUnit` semantics, polkit default, verification
criteria, deadline, idempotency model, or allowed D-Bus surface requires an ADR
that supersedes this one.

## References

- The current systemd
  [`org.freedesktop.systemd1` source manual](https://github.com/systemd/systemd/blob/main/man/org.freedesktop.systemd1.xml)
  defines `TryRestartUnit`, job objects/signals, and unit state properties.
- The current
  [`systemctl` source manual](https://github.com/systemd/systemd/blob/main/man/systemctl.xml)
  documents that try-restart does nothing for an inactive unit and describes
  restart side effects.
- The current
  [`systemd.exec` source manual](https://github.com/systemd/systemd/blob/main/man/systemd.exec.xml)
  defines service filesystem, namespace, privilege, and process restrictions.
- ADR-0009 defines the system-bus caller, polkit, idempotency, audit, and
  fail-closed boundary inherited by this operation.
