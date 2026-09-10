# Phase 11 physical installation guard

Status: implemented and tested with one bounded disposable-media writer. No
physical installer or internal-disk write authority is enabled.

The guard accepts one closed, purpose-bound observation containing the
successful host preflight outcome, AC-power and recovery-media readiness, a
per-attempt challenge, the live-medium device, and at most eight bounded disk
summaries.

Unavailable zero-capacity card-reader slots are excluded before normalization;
they contain no addressable media and cannot be a live device or target. Every
non-empty disk remains subject to the complete closed validation and selection
rules below.

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
Checkpoint 4 completed on 2026-09-10: the separately reviewed real
disposable-media run completed and restored its bounded region. Cancellation,
ambiguity, unplug or changed-device rejection, failure recovery, and
exact-target behavior are covered by deterministic tests. Physical installation
remains a separate unauthorized checkpoint.

The next reviewed increment uses a distinct physical-install harness. It
accepts only `physical_install` authority, repeats the exact confirmation and
target-digest comparison against a fresh observation, consumes an exclusive
once-only claim before invoking a backend, and refuses disposable-test
authority. Backend failure remains terminal for that claim. The command runner
uses one fixed absolute backend path, an argument vector without a shell, a
bounded timeout, and only the guard-selected device path. This establishes the
execution boundary but does not yet provide or authorize the destructive
backend.

## Bounded disposable-media probe

The next writer increment is intentionally not an installer. After the existing
guard and once-only harness accept an unchanged `disposable_test` target,
`disposable_media_probe.py` opens that exact block device with `O_EXCL` and
`O_NOFOLLOW`, confirms its live byte size with `BLKGETSIZE64`, and touches only
4 KiB at the fixed 8 MiB offset. It reads the original region, writes a
target-bound deterministic marker, flushes and reads it back, then restores,
flushes, and reads back the original bytes before reporting success.

The probe rejects files, symlinks, mounted or otherwise busy devices, changed
sizes, non-USB authority, and `physical_install` authority. It returns no
original bytes or content-derived fingerprint. Failure after a marker attempt
always enters the restore path; a restore failure is reported explicitly and
the once-only claim remains consumed. The command still requires root to open a
real block device, so execution remains a separate manual, action-time-approved
step on the frozen host.
