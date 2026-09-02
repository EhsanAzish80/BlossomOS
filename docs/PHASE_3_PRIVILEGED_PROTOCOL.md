# Phase 3 privileged protocol foundation

Status: implemented and locally verified on 2026-09-02. This checkpoint defines
shared data and verification only. It does not register a tool, contact polkit,
install a system service, or expose a privileged operation.

## Closed contract

`core/blossom-core/src/privileged.rs` owns the version-1 constants and types for
ADR-0010's fixed Bluetooth recovery request. The request can carry only a
bounded correlation ID, a 128-bit lowercase-hex idempotency key, and the
interactive flag. The unit, method, job mode, bus identifiers, and polkit action
are compile-time constants and cannot be supplied by a caller.

The canonical SHA-256 request digest binds those constants, the complete request,
and the helper-authenticated UID. Changing caller identity, correlation,
idempotency, interactivity, or any fixed operation value produces a different
digest.

## Result verification

The shared verifier rejects unknown fields, unsupported versions, correlation or
digest mismatch, invalid fixed provenance, unbounded state tokens, impossible
job-submission claims, and false success. `restarted_active` requires:

- exact canonical `bluetooth.service` and `loaded` state before and after;
- active state before and after;
- non-zero 16-byte invocation identities that differ; and
- exact job result `done`.

An inactive observation may legitimately have a zero invocation identity and is
accepted only as `not_running`, never restart success. Pre-authorization errors
cannot claim that a job was submitted; job, verification, and indeterminate
failures must truthfully record that submission occurred.

## Tests and non-authority

Portable tests cover closed schema rejection, bounds, versioning, normalized
digest binding, successful independent verification, unchanged invocation
rejection, and impossible failure-state rejection. The existing complete Rust
and repository suites remain green.

These serializable types are not authorization. No request variant, policy rule,
CLI command, D-Bus client, privileged helper, activation file, polkit action,
systemd unit, journal, or host mutation is added by this checkpoint. Those
pieces must arrive together under ADR-0009 and ADR-0010's implementation gate.
