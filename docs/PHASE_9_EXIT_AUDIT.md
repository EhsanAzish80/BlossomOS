# Phase 9 exit audit

Status: corrected on 2026-09-27; update/rollback exit reopened.

## 2026-09-27 correction

The historical evidence proved a signed file-slot lifecycle and separately
proved that one installed root filesystem booted. It did not connect slot
selection to independently bootable system roots. The prior A/B rollback pass
therefore overstates the evidence. Installation evidence remains historical
evidence; bootable update, rollback, and recovery are open pending ADR-0029 and
new qualification. No physical-device readiness follows from the old result.

## Accepted boundary

Phase 9 establishes a Blossom-owned distribution path from reviewed source to
closed Arch packages, a pinned ArchISO, a fresh blank-disk UEFI installation,
and a signed offline A/B update and recovery lifecycle. The installed system
keeps network updates, unattended updates, SSH, telemetry, and runtime model
downloads disabled.

The evidence target is exactly an x86-64 UEFI virtual machine on the trusted
self-hosted Linux runner. This audit does not claim a public release, daily-use
readiness, physical-device support, another architecture, legacy BIOS, Secure
Boot, disk encryption, dual boot, public signing-key readiness, or broad
hardware compatibility.

## Exit evidence

| Gate | Evidence | Result |
| --- | --- | --- |
| Closed distribution boundary | ADR-0026, `distribution/manifest.json`, pinned snapshot and container digests | Pass |
| Real Blossom packages | `makepkg` builds `blossom-core` and `blossom-shell`; the image workflow installs both artifacts into the root filesystem and inspects them after disk boot | Pass |
| Blossom-owned installation image | The reviewed ArchISO profile builds without mutable runtime downloads, development credentials, autologin, enabled SSH, or telemetry | Pass |
| Fresh UEFI installation | [Run 34218749512](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34218749512) boots the ISO, partitions a blank disk, installs the packaged root filesystem, installs systemd-boot, then boots the disk | Pass |
| Signed lifecycle model | [Run 34196824642](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34196824642) verifies signed metadata and file-slot state transitions with network denied | Limited: not bootable A/B |
| Installed lifecycle | Run 34218749512 rechecks installed packages and emits lifecycle and disk-boot markers | Limited: slot selection did not control boot roots |
| Adversarial behavior | The Python lifecycle suite rejects unsupported hardware, unknown metadata, expiry, downgrade, active-slot replacement, mutation, invalid recovery, and unsafe first-run transitions | Pass |
| Independent repository gates | Distribution validation, repository policy, unit tests, formatting, linting, dependency review, CodeQL, and secret scanning are protected checks | Pass |

The implementation evidence head is
`258b74e78459379e62efe2bdb8bb9565b672ddb8`. The generated
`blossom-os-0.9.0-evidence-x86_64.iso` has SHA-256
`f9a501e5b5d22e3805176cb9e08b51864e6e9cba3c08c0e6c7439a8d1829e233`.
The runtime workflows bind their checkout to the reviewed commit. Evidence data
is isolated beneath the runner temporary directory and removed on completion.

## Exit decision

The package, image, and single-root installation evidence remains retained, but
the Phase 9 bootable update/rollback gate is reopened. Distribution and updates
remain pre-alpha research foundations, not a supported installer or release.
