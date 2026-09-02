# Phase 3 privileged-operation baseline

Status: complete. The exit review passed all checks and merged to `main` in
PR #34 as `0662e51`. Phase 3's adversarial evidence checkpoint passed and
merged in PR #33 as `8cfb9b9`.

Blossom OS remains pre-alpha. The Phase 3 implementation proves one narrowly
typed privileged boundary; it is not a general-purpose root automation system
and has not been validated on real Bluetooth hardware or a target Arch install.

## Implemented boundary

The only privileged request is `TryRestartBluetooth1`. Its code-owned operation
is `TryRestartUnit("bluetooth.service", "replace")`. The caller can supply only
the protocol version, a bounded correlation identifier, a 128-bit idempotency
key, and the interactive flag. It cannot supply a command, executable,
arguments, unit, action, bus address, object path, systemd method, or job mode.

The end-to-end path is:

```text
interactive CLI approval
  -> fixed system-bus method
  -> authenticated sender and UID capture
  -> fixed polkit action
  -> durable idempotency claim
  -> fixed native systemd call
  -> result verification
  -> synced hash-chained audit and terminal journal state
```

No permanent approval is implemented. Non-interactive use, denial,
cancellation, expiry, invalid callers, and unavailable authorization fail
closed.

## ADR-0010 implementation-gate evidence

| Requirement | Repository evidence |
| --- | --- |
| Closed shared schemas and byte fixtures | `core/blossom-core/src/privileged.rs` validates closed bounded types and exact serialized request/failure fixtures. |
| Exact method mapping | `system/privileged-helper/src/system_service.rs` exports one method and its private-bus test rejects `Execute`. Packaging policy permits only that member. |
| Exact interactive approval | `apps/blossom-cli/tests/privileged_bluetooth_flow.rs` proves the preview and once-only approval precede helper contact; denial, cancellation, expiry, and non-TTY paths contact no helper. |
| Authenticated caller and polkit binding | The service captures the D-Bus header sender, resolves its Unix UID, rejects invalid/root callers, and the polkit adapter uses only the `system-bus-name` subject and fixed action/details. |
| Replay and journal durability | File-journal tests cover synced state recovery, duplicate handling, digest mismatch, interruption states, corruption, unsafe modes, symlinks, stale replacements, capacity, and transition failures. Helper tests prove no double execution and conservative outcomes after possible submission. |
| Controlled polkit service | Target-Linux private-D-Bus tests cover exact subject/action/details, denial, challenge, dismissal, timeout, invalid/noninteractive subjects, and caller disconnect. |
| Controlled systemd service | Target-Linux private-D-Bus tests cover exact GetUnit/properties/TryRestartUnit use, changed invocation, wrong-job filtering, wrong-job timeout, unavailable unit, malformed state, and missing bus. Helper tests cover inactive state, terminal job failure, operation timeout, and failed verification. |
| Zero jobs on negative approval paths | CLI integration tests prove denial, cancellation, expiry, and non-TTY paths make zero helper calls; helper tests prove denial and invalid pre-state make zero manager calls. |
| Payload cannot expand authority | Closed serde types reject unknown fields; operation constants and adapter destinations are code-owned; the request has no generic resource or execution fields. |
| Packaging | `scripts/ci/check_privileged_packaging.py` checks the systemd unit, D-Bus activation and member policy, polkit action, fixed paths, ownership/mode declarations, and forbidden shell/privilege launchers. CI also runs `systemd-analyze verify`. |
| Audit | The helper records correlated transitions; the file backend is bounded, synced, hash-chained, recovery-validated, and fail-closed on tampering, truncation, unsafe modes, or write failure. |
| Rust and security checks | CI runs formatting, clippy with warnings denied, all-target tests on Linux, repository policy checks, ShellCheck, Gitleaks, CodeQL, and dependency review. The Rust crates forbid unsafe code. |

CI uses private controlled D-Bus services and never restarts the runner's real
Bluetooth service.

## Independent generic-root-path review

Production Rust contains no `Command::new`, shell launcher, generic executable
request, generic systemd method, or generic privileged D-Bus method. The only
production systemd mutation string is `TryRestartUnit`, bound to the fixed
Bluetooth unit and `replace` mode. `Command::new("dbus-daemon")` occurs only in
target-Linux test modules used to create controlled private buses. The
`Execute("/bin/sh")` occurrence is an unknown-method rejection test.

The preserved pre-agent prototype still contains historical shell installers,
sudo guidance, and an XFCE-era rule-based assistant. Those files are not part of
the trusted Rust privileged-helper path and remain non-production prototype
material as documented in the README and Phase 0 baseline. Their presence must
not be interpreted as a supported privileged interface.

## Deferred evidence and limits

- The helper packaging is validated in CI but is not installed by this repository.
- Real target-Arch installation and Bluetooth hardware behavior remain an
  explicit later test gate.
- The helper does not provide Bash, sudo, package management, arbitrary service
  control, a model runtime, or a graphical shell.
- Phase 4 must not widen this boundary or treat model output as authorization.

## Completion decision

Phase 3 is complete. The checkpoint passed the protected Linux test, lint,
packaging, secret, static-analysis, and dependency-review checks before merge.
The repository dependency graph was enabled so dependency review runs for real,
and `Dependency review` is now a required `main` branch check. Private
vulnerability reporting, secret scanning, push protection, strict up-to-date
required checks, and administrator enforcement were verified enabled at exit.
