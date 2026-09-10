# ADR-0028: Phase 11 first physical-device qualification boundary

- Status: Accepted
- Date: 2026-09-10
- Owners: Blossom OS maintainers

## Context

Phases 9 and 10 proved a reviewed x86-64 UEFI virtual-machine target. They did
not prove a safe physical installer or supported hardware. The existing
evidence installer is intentionally bound to `/dev/vda` and automatically
repartitions that blank virtual disk. Reusing it on a physical machine would be
unsafe.

The first disposable physical evidence target is one expected Apple
`MacBookPro11,1`. Its exact identity must be confirmed by the preflight rather
than inferred from its appearance, hostname, or prior operating system.

## Decision

Phase 11 begins with a read-only preflight for exactly `MacBookPro11,1`,
`x86_64`, UEFI, at least 4096 MiB memory, DRM graphics, and an internal disk.
An eligible result means only that qualification may proceed. It grants no installation authority and makes no compatibility or support claim.

The preflight uses a closed schema and emits no serial number, MAC address,
SSID, filesystem UUID, username, or disk path. The VM installer remains
unchanged and VM-only.

Before any later physical write is implemented, a separate reviewed increment
must:

- disable automatic installation on physical boot media;
- enumerate and display the exact target model, capacity, connection type, and
  whether the device is removable, without stable identifiers;
- reject the live medium, mounted devices, ambiguous targets, unexpected
  devices, and power or recovery-precondition failures;
- require an explicit typed confirmation bound to the reviewed target summary;
- preserve a recoverable failure path and never infer consent from preflight;
- pass destructive tests only against disposable, independently identified
  media before use on the frozen laptop.

Physical qualification then covers UEFI boot, internal installation and boot,
display/DRM, keyboard, trackpad, network, audio, power/battery, suspend/resume,
update rollback, and recovery. Each outcome is recorded independently.

## Alternatives considered

- **Run the VM installer on the laptop.** Rejected because its automatic disk
  destruction is safe only inside the reviewed blank VM.
- **Treat the working Linux installation as Blossom evidence.** Rejected
  because it proves host availability, not Blossom installation or lifecycle.
- **Declare Intel Mac support from one laptop.** Rejected because one exact
  device cannot establish a family-wide support claim.
- **Collect a complete hardware inventory.** Rejected because stable device
  identifiers and unrelated hardware detail are unnecessary for this gate.

## Security and privacy consequences

The initial increment cannot write disks, alter firmware, install software, or
grant runtime capability. Its minimized output can identify the reviewed model
class but not a particular owner or device. Malformed and expanded observations
fail closed.

## Migration and rollback

This increment adds only documentation, validation, and a pure classifier. It
can be removed without affecting installed systems. Any future physical
installer must be independently reviewed and can be withheld while VM evidence
continues unchanged.

## Validation

Unit tests cover the exact target, wrong architecture, firmware, model, memory,
graphics and storage, schema expansion, type confusion, output minimization,
and continued VM-only installer binding. The first real laptop run remains a
separate physical evidence checkpoint.
