# Phase 11 physical installation guard

Status: implemented and deterministically tested as a non-writing guard. No
physical installer or disk-write authority is enabled.

The guard accepts one closed, purpose-bound observation containing the
successful host preflight outcome, AC-power and recovery-media readiness, a
per-attempt challenge, the live-medium device, and at most eight bounded disk
summaries.

`physical_device_observer.py` supplies that inventory on Linux through one
fixed, read-only `lsblk` invocation. It accepts at most 64 KiB and eight whole
disks, recognizes only SATA, NVMe, and USB transports, and requires exactly one
live root (`/`) or live-media (`/cdrom`) disk. It collects no serial number,
filesystem label, UUID, partition content, or user data. Power and recovery
readiness remain explicit operator assertions; the observer does not infer
them.

The manually dispatched `Phase 11 disposable device observation` workflow runs
the frozen-host preflight, device observer, and non-writing guard on the trusted
physical runner. It requires explicit AC-power and recovery-media assertions
and uploads only the minimized preflight, observation, and guard-decision JSON.
It has no `sudo`, device writer, partitioner, formatter, mount operation, or
automatic trigger.

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
- device-observer output is excessive, incomplete, unsupported, or cannot
  identify exactly one live disk.

## Next gate

The deterministic harness now provides once-only consumption, cancellation,
independent device revalidation, truthful post-claim failure, and an injected
fake backend. It rejects `physical_install` authority before invoking its
backend; only a purpose-bound `disposable_test` decision can enter this harness.
Checkpoint 4 still requires a separately reviewed real disposable-media run.
No physical laptop write may occur before cancellation, ambiguity, unplug,
mutation, and failure-recovery tests pass there.
