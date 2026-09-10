# Phase 11 physical installation guard

Status: implemented and deterministically tested as a non-writing guard. No
physical installer or disk-write authority is enabled.

The guard accepts one closed, purpose-bound observation containing the
successful host preflight outcome, AC-power and recovery-media readiness, a
per-attempt challenge, the live-medium device, and at most eight bounded disk
summaries.

For `physical_install`, it fails closed unless there is exactly one
non-removable, unmounted internal SATA or NVMe target distinct from the live
medium. For `disposable_test`, it accepts only an unmounted USB target. Purpose,
device path, model, size, transport, and the per-attempt challenge are bound
into a canonical SHA-256 digest. The operator must type the exact displayed
phrase:

```text
ERASE <device-path> <first-16-digest-characters>
```

Case changes, whitespace, a different path, a different challenge, or any
target mutation prevents the guard from passing. Even a passing result is
`guard_passed_no_write_performed`: this module contains no partitioning,
formatting, mounting, or installation operation.

## Rejection matrix

- host preflight is not eligible;
- AC power or recovery media is not ready;
- malformed, unknown, duplicate, unsupported, or excessive device records;
- live medium is absent or selected as the target;
- candidate is removable or mounted;
- zero or multiple eligible internal targets;
- target is outside bounded size, path, model, or transport rules;
- typed confirmation differs from the exact bound phrase.
- purpose changes between disposable testing and physical installation.

## Next gate

The deterministic harness now provides once-only consumption, cancellation,
independent device revalidation, truthful post-claim failure, and an injected
fake backend. Checkpoint 4 still requires a separately reviewed real
disposable-media run.
No physical laptop write may occur before cancellation, ambiguity, unplug,
mutation, and failure-recovery tests pass there.
