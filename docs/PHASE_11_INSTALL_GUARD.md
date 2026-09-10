# Phase 11 physical installation guard

Status: implemented and deterministically tested as a non-writing guard. No
physical installer or disk-write authority is enabled.

The guard accepts one closed observation containing the successful host
preflight outcome, AC-power and recovery-media readiness, a per-attempt
challenge, the live-medium device, and at most eight bounded disk summaries.

It fails closed unless there is exactly one non-removable, unmounted internal
SATA or NVMe target distinct from the live medium. Device paths, model, size,
transport, and the per-attempt challenge are bound into a canonical SHA-256
digest. The operator must type the exact displayed phrase:

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

## Next gate

Checkpoint 4 must connect this guard to a separately reviewed disposable-media
test harness with once-only consumption and independent device revalidation.
No physical laptop write may occur before cancellation, ambiguity, unplug,
mutation, and failure-recovery tests pass there.
