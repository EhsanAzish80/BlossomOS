# Phase 3 portable helper core

Status: implemented and locally verified; not installed or exposed.

`system/privileged-helper` contains the portable security state machine for the
single operation accepted by ADR-0010. It accepts only the closed
`BluetoothRestartRequest` type and delegates four separately testable boundaries:
independent authorization, the fixed Bluetooth manager operation, an idempotency
journal, and redacted helper audit events.

The ordering is security relevant: validate, authorize, claim the request digest,
observe the fixed unit, durably mark submission, perform the fixed operation,
verify the returned evidence, and complete the journal. A completed duplicate is
returned without another operation. Recovery of a claim that never reached the
submission marker reports `interrupted_before_submission`; recovery at or after
that marker reports `outcome_indeterminate`. Neither state is automatically
retried.

The in-memory journal and audit implementations exist only as deterministic test
doubles. The Unix `FileJournal` is the durable backend intended for the future
systemd `RuntimeDirectory`: it requires a pre-created root-owned `0700`
directory, uses only `0600` regular files opened without following symlinks,
bounds entries and post-write bytes, syncs each transition, and atomically
replaces later states. Corruption, stale temporary files, unknown directory
entries, permission drift, digest reuse, and invalid transitions fail closed.
Tests substitute the current test user's UID while enforcing the same modes.

The durable backend is not yet wired into an executable or installed runtime
directory. No D-Bus server or client, polkit call, systemd mutation adapter,
packaging, service unit, activation policy, or CLI registration is present at
this checkpoint. The crate therefore cannot restart a service and is not a
security claim about an installed privileged boundary.

The Unix `FileAudit` backend similarly requires its own pre-created trusted
`0700` directory and maintains a synced `0600` JSON-lines log. Each bounded
record contains a monotonic sequence, the previous digest, a closed redacted
event, and the resulting digest. Recovery validates the complete chain and
rejects truncation, tampering, symlinks, unexpected files, permission drift,
record overflow, and byte overflow. The sequence and record digest provide the
local audit identity; exposure through the future broker activity view remains
unimplemented.

Local evidence for this checkpoint is:

- workspace formatting passes;
- all workspace tests pass, including denial, inactive-unit, successful replay,
  changed-digest rejection, timeout after submission, recovered journal states,
  durable transition recovery, journal corruption/containment, and audit
  persistence/tamper cases; and
- strict workspace Clippy passes with warnings denied.

Target-Linux integration and installed-boundary evidence remain required before
the Phase 3 implementation item or exit gate can be marked complete.

## Fixed systemd adapter

On GNU/Linux, `SystemdBluetoothManager` implements the manager boundary with
native zbus calls only. Its production constructor fixes the local system bus,
20-second deadline, systemd destination/path/interface, `bluetooth.service`,
`TryRestartUnit`, and `replace` mode in code. It subscribes to the fixed
`JobRemoved` signal before submission, matches the returned job object path,
accepts only bounded terminal categories, and re-reads the closed unit property
set including the 16-byte invocation ID. It has no subprocess or configurable
unit, method, mode, address, or argument surface.

The target-Linux test uses a private mock D-Bus address compiled only in the test
configuration. It proves the exact pre-observation, single restart submission,
matching completion, and post-observation sequence without touching the runner's
real Bluetooth service. A missing bus produces only a disconnected/timeout
error. Installation, system-bus service exposure, polkit authorization, and a
real privileged mutation remain separate gates.

## Fixed polkit adapter

On GNU/Linux, `PolkitAuthorizer` calls only the system polkit Authority's
`CheckAuthorization` method. It constructs a `system-bus-name` subject from the
helper-authenticated unique sender, fixes the action to
`org.blossomos.privileged1.try-restart-bluetooth`, supplies only bounded
operation/unit/correlation details, requests one interactive authorization, and
does not request retained authority. Invalid senders and noninteractive requests
are denied before bus contact; authority errors fail unavailable and the human
authorization deadline expires closed.

The adapter accepts no PID, process name, executable, action identifier,
authorization token, unit, or arbitrary detail from the caller. Target-Linux
tests verify the exact subject, action, details, and flags against a private mock
Authority. The following system-service checkpoint captures the real method
sender, resolves its UID through the bus, rechecks ownership after authorization,
and exposes the one helper method.

## System service and package boundary

The GNU/Linux binary now constructs only the concrete fixed polkit authorizer,
Bluetooth systemd adapter, durable idempotency journal, and durable audit log.
Its D-Bus object exports one method, `TryRestartBluetooth1`, with four bounded
protocol fields. The service captures the message's unique sender, resolves its
Unix UID through the bus, rejects missing credentials and root callers, and
serializes requests through one handler lock. Unknown methods never enter the
handler. Results are bounded closed-schema JSON bytes for the version-1 client.

The polkit adapter rechecks `NameHasOwner` for the exact unique sender after a
successful authorization; a disconnected caller cannot proceed into the
journal or systemd operation. Target-Linux controlled-bus tests prove real
sender/UID capture, fixed action binding, disconnect rejection, one-method
dispatch, and unknown-method rejection.

`packaging/` defines the intended root-owned binary, Type=dbus systemd unit,
system-bus activation and exact-member policy, and one polkit XML action with
`no`/`no`/`auth_admin` defaults. The service has empty capability and ambient
sets, AF_UNIX-only IPC, no IP networking, a strict read-only host view except
its boot runtime, and standard namespace/kernel/device/process hardening. A CI
validator pins these identifiers and rejects polkit rules or generic execution
surfaces. The files are not installed by tests and no release package exists.
