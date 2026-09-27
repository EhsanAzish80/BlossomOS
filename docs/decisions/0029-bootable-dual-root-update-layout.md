# ADR-0029: Bootable dual-root update layout

- Status: Accepted
- Date: 2026-09-27
- Accepted: 2026-09-27 after project-owner review
- Owners: Project maintainers
- Effect: supersede the file-backed slot implementation
  described by ADR-0026; the remaining package, signature, first-run, and
  recovery boundaries remain in force.

## Context

ADR-0026 requires two immutable system payload slots, inactive-slot staging,
health confirmation, and automatic rollback. The Phase 9 lifecycle prototype
models those slots as files under one directory. The Phase 11 physical installer
creates one EFI system partition and one ext4 root partition. Changing the
prototype's `active-slot` file therefore cannot change which operating system
the firmware and bootloader start.

The physical layout, boot selection, update verifier, recovery media, and health
confirmation must describe the same real state machine before further installer
hardening or physical qualification can be meaningful.

## Decision

### Disk layout

The first supported blank-disk installation uses GPT with four partitions:

1. `BLOSSOM_EFI`: 1 GiB FAT32 EFI system partition.
2. `BLOSSOM_ROOT_A`: 24 GiB ext4 system slot A.
3. `BLOSSOM_ROOT_B`: 24 GiB ext4 system slot B.
4. `BLOSSOM_STATE`: the remaining space as ext4 persistent state.

The installer rejects disks too small to provide both complete root slots plus
at least 16 GiB of state. It discovers every resulting partition from a fresh
bounded block inventory; it never constructs `/dev/sdXN` names.

Both root slots are complete independently bootable systems. Mutable directory
state lives on `BLOSSOM_STATE`, including `/home`, Blossom durable state,
update state, persistent qualification logs, saved NetworkManager connections,
and Bluetooth pairings. Those directories are bind-mounted into either active
root.

Files that system tools replace atomically remain ordinary files in each root.
The closed synchronization set contains the account databases, hostname,
machine ID, locale and console settings, timezone link, and generated locale
archive. A missing source path removes its inactive-slot counterpart because
absence is part of the synchronized state. Installation provisions slot A,
generates a fresh installation-specific machine ID from an empty image
placeholder, and copies the closed set into slot B. A
physical updater must copy the same set from the running confirmed root into
the newly written inactive root before selecting its trial UKI, then fsync each
replaced file and parent directory before changing boot selection. If state
fails to mount, either root therefore retains usable local account data for
repair.

Account or machine-setting changes made during a trial boot are not copied back
to the confirmed root and can be lost if the trial rolls back. This is an
explicit A/B trade-off for this phase; settings should be changed after trial
promotion.

The image build forces `/etc/machine-id` to an empty, read-only placeholder and
rejects a non-empty value. It emits a small closed manifest recording the
required account and UKI inputs and zero machine-ID bytes; the checksum file
authenticates that manifest alongside the rootfs archive. Before the first
destructive command, installation requires graphical account provisioning and
validates the already-verified manifest without decompressing the archive
again. The installer writes a fresh 128-bit random machine ID directly, so VM
firmware identifiers cannot make separate installs share an identity. The
command-line unprovisioned disk-wipe route is not shipped.

### Boot artifacts and selection

Each slot owns one versioned unified kernel image on `BLOSSOM_EFI`. The UKI
binds its root filesystem by filesystem UUID. The persistent systemd-boot
default remains the last confirmed slot.

An update writes and verifies the inactive root partition and its inactive UKI
before changing boot state. It copies the closed slot-local identity and locale
set from the running confirmed root into the inactive root, then verifies the
result. It then selects that UKI for a bounded trial boot
using systemd-boot's one-shot and boot-counting facilities. Failure to confirm
health exhausts the trial entry and returns to the still-confirmed persistent
default. Successful health confirmation makes the new entry persistent and
retires the previous slot only as the next inactive update target.

If EFI variables, boot counting, the confirmed entry, or the inactive slot
cannot be observed unambiguously, the update stops before changing boot
selection.

### Verified update object

Metadata is read once into bounded immutable bytes. Signature, closed-schema,
identity, expiry, and sequence validation operate on those exact bytes. The
payload is opened once without following symlinks and retained by descriptor;
the updater streams that same open object to the inactive slot while hashing it.
Size and digest must match before any boot-selection change. Staging cannot
reopen caller-selected metadata or payload paths. The small Phase 9 evidence
prototype may retain its complete payload in memory, but the physical updater
must use the retained-descriptor streaming form.

### Recovery

Recovery media identifies an installation by its marker and filesystem UUIDs.
It verifies both UKIs and both root slots before offering a boot-selection
repair. It can select only a verified slot belonging to that installation. An
attached device or label proves only that recovery media is present; physical
bootability remains a separate observed qualification gate.

## Alternatives considered

### Btrfs snapshots

Rejected for the first physical target. Snapshots reduce duplicated system
storage, but safe rollback also depends on subvolume layout, bootloader command
lines, snapshot retention, writable-state separation, and garbage collection.
Those extra coupled mechanisms make the first auditable physical rollback path
harder to reason about.

### File-backed slots on one ext4 root

Rejected. Switching a marker file does not alter firmware or bootloader system
selection and therefore cannot satisfy the rollback claim.

### In-place package upgrades

Rejected for this phase. Interruption can leave the only bootable root partially
updated, and rollback cannot be made truthful without a separately verified
system copy.

## Security and privacy consequences

The inactive slot and UKI remain untrusted until their exact bytes have been
verified and durably written. The confirmed slot is not modified during staging.
User data is outside both system slots and is not erased by system rollback.

The EFI partition becomes security-critical update state. Entry names, UKI
paths, filesystem UUIDs, sequence numbers, and slot identities are closed,
code-owned values. Recovery and update code reject ambiguous devices, unknown
entries, mounted update targets, symlinks, and identity changes between
observation and execution.

Full-disk encryption, Secure Boot key enrollment, dual boot, migration from the
prototype layout, and rollback across incompatible state-schema migrations
remain unsupported and must not be implied by this decision.

## Operational consequences

The layout consumes 48 GiB for system roots on the first 121 GB target. In
exchange, a confirmed complete root remains bootable during every inactive-slot
write. Image assembly must produce per-slot root payloads and UKIs. Installer,
updater, recovery, boot-health, qualification, and disk-observation evidence
must share one slot schema.

Persistent logs live on `BLOSSOM_STATE`; qualification builds may not force
`journald` to volatile storage.

## Migration and rollback

There is no in-place migration from the existing single-root physical candidate.
That candidate remains unsupported evidence media. A new installer must perform
a fresh installation onto an explicitly authorized blank target.

Before release, reverting this proposal means returning to no supported update
or rollback claim. It does not permit restoring the file-backed slot claim.

## Validation

Acceptance requires evidence that:

- partition discovery uses stable observed identities and never constructed
  `/dev/sdXN` paths;
- power loss at every partition, root, UKI, metadata, and boot-selection write
  leaves the confirmed slot bootable;
- metadata and payload mutation after verification cannot affect staged bytes;
- the first failed trial boot returns to the confirmed slot without changing
  user data;
- health confirmation alone promotes the trial slot to persistent default;
- recovery rejects foreign, ambiguous, corrupt, or unmarked installations;
- both slots boot independently after installation; and
- physical hardware qualification remains separate from VM evidence.
