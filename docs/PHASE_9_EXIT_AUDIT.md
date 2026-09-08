# Phase 9 exit audit

Status: complete on 2026-09-08 for the reviewed x86-64 UEFI VM evidence
target.

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
| Signed lifecycle | [Run 34196824642](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34196824642) verifies canonical Ed25519-signed offline metadata, inactive-slot staging, rollback, confirmation, recovery, and user-data preservation with network denied | Pass |
| Installed lifecycle | Run 34218749512 rechecks the installed packages and emits rollback, confirmation, recovery, and disk-boot verification markers from the installed disk | Pass |
| Adversarial behavior | The Python lifecycle suite rejects unsupported hardware, unknown metadata, expiry, downgrade, active-slot replacement, mutation, invalid recovery, and unsafe first-run transitions | Pass |
| Independent repository gates | Distribution validation, repository policy, unit tests, formatting, linting, dependency review, CodeQL, and secret scanning are protected checks | Pass |

The implementation evidence head is
`258b74e78459379e62efe2bdb8bb9565b672ddb8`. The generated
`blossom-os-0.9.0-evidence-x86_64.iso` has SHA-256
`f9a501e5b5d22e3805176cb9e08b51864e6e9cba3c08c0e6c7439a8d1829e233`.
The runtime workflows bind their checkout to the reviewed commit. Evidence data
is isolated beneath the runner temporary directory and removed on completion.

## Exit decision

All Phase 9 gates pass for the accepted evidence target. Distribution and
updates remain pre-alpha research foundations, not a supported installer or
release. Public-beta hardening, release signing and publication, SBOM work,
broader threat review, and supported-hardware claims belong to Phase 10.
