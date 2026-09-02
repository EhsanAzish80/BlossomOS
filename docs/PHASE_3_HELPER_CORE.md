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
doubles. They are not suitable for the root service. No D-Bus server or client,
polkit call, systemd mutation adapter, persistent root-owned journal, packaging,
service unit, activation policy, or CLI registration is present at this
checkpoint. The crate therefore cannot restart a service and is not a security
claim about an installed privileged boundary.

Local evidence for this checkpoint is:

- workspace formatting passes;
- all workspace tests pass, including denial, inactive-unit, successful replay,
  changed-digest rejection, timeout after submission, and recovered journal
  states; and
- strict workspace Clippy passes with warnings denied.

Target-Linux integration and installed-boundary evidence remain required before
the Phase 3 implementation item or exit gate can be marked complete.
