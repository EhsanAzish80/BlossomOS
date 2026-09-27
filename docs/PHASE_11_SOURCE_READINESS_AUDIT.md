# Phase 11 source-readiness audit

- Date: 2026-09-27
- Scope: source and documentation review only
- Build performed: no
- Automated tests performed: no
- VM or physical boot performed: no
- Release status: blocked pending the evidence matrix below

## Closed in source

- ADR-0029 is accepted. The physical installer now lays out a 1 GiB EFI
  partition, two 24 GiB ext4 root slots, and a persistent state partition.
- Partition nodes are rediscovered by GPT partition label after udev settles;
  the installer no longer constructs `/dev/sda1`-style child names.
- The destructive disk is opened through an observed `/dev/disk/by-id` alias,
  retained for the operation, and rechecked for block type and exact size.
- Both roots receive a complete system. `/home`, `/var/lib/blossom`,
  `/var/log`, account databases, machine identity and hostname, saved network
  connections, and Bluetooth pairings are placed on the shared state
  partition. Both slots receive a slot-bound UKI and systemd-boot entry.
- The embedded root archive has a separately embedded SHA-256 digest and is
  verified before the first destructive command. Because archive and digest
  are carried by the same ISO, this detects accidental corruption only; ISO
  provenance/signature verification remains the authenticity boundary.
- Update evidence code reads metadata and payload once and stages the same
  verified bytes, closing its metadata/payload path-reopen race.
- Live safety input no longer accepts typed power or recovery-readiness claims.
  AC state comes from the kernel power-supply inventory. The narrowly named
  `live_media_present` field reports only the uniquely identified ArchISO disk;
  it is not described as proof that separate recovery media will boot.
- Physical-host preflight observes the frozen BCM4360 PCI identity. The
  installed root obtains `broadcom-wl-dkms` directly from the pinned signed
  Arch snapshot through `pacstrap`; no unsigned local repository is added.
  The offline live installer does not add DKMS or a second kernel solely for
  Wi-Fi. NetworkManager uses explicit `wpa_supplicant`; unused iwd is removed.
- Every physical and VM-qualification build fails closed unless the closed
  Qwen2.5 0.5B llama.cpp runtime/model package tree is supplied through
  `BLOSSOM_LLAMA_CPP_RUNTIME_ROOT`. The package profile, lock digest, gateway,
  provider unit, runtime files, and model file are rehashed before copying. It
  enables only the reviewed
  provider, private namespace, and model-gateway units.
- Release assembly copies an explicit runtime-module allowlist. Tests,
  workflows, repository metadata, and general build scripts are not installed.
- Installed journald storage is persistent. Qualification logs can therefore
  survive a reboot on the state partition.
- Installed-account provisioning remains graphical, creates the chosen user,
  hashes the chosen password, grants the reviewed wheel/sudo policy, and uses
  greetd/ReGreet rather than a fixed installed autologin account.
- Expired approval records and consumed replay tombstones are swept, preventing
  unbounded in-memory growth.
- ADR-0021 now states the actual limit of the current approval channel: D-Bus
  connection binding proves process continuity, not human intent. Sensitive
  capabilities remain blocked until a separately reviewed trusted-input
  ceremony exists.

## Still blocked on implementation or evidence

- ADR-0029's physical inactive-slot updater, boot-counted trial promotion,
  automatic rollback, and recovery repair path are not yet wired to the new
  partitions. The Phase 9 file-backed lifecycle remains evidence code only and
  must not be presented as physical A/B update support.
- The two-slot installer and UKI path have not been built, syntax-checked,
  installed, or booted in this audit.
- The pinned snapshot's Broadcom package and the closed llama.cpp/model package
  tree have not yet been assembled or verified in an image.
- BCM4360 association, DHCP, DNS, TLS, suspend/resume, Bluetooth coexistence,
  audio, graphics, scaling, login, installer completion, and local inference
  require fresh physical evidence on the frozen MacBookPro11,1.
- A bootable recovery device can only be established by a separate observed
  boot. Its presence check is intentionally narrower.
- Approval for sensitive capabilities remains architecturally blocked; token
  cleanup does not solve trusted human presence.
- No source-only review can establish power-loss safety. The accepted A/B
  design requires interruption testing at each destructive and boot-selection
  boundary before any update/rollback claim.

## Required next evidence sequence

1. Review this source diff and reconcile the intentionally preserved branch.
2. Run static syntax and focused unit checks without producing an ISO.
3. Assemble all offline inputs and record their exact digests.
4. Build one local candidate once, then inspect its filesystem and package
   inventory before booting it.
5. Qualify install, independent boots of A and B, trial failure rollback,
   promotion, recovery rejection/repair, persistent logs, and local inference
   in a disposable VM.
6. Only after the VM gate passes, perform a separately approved physical test
   with fresh disk inventory and a new action-time erase confirmation.

Until every applicable item is observed, this branch is source work in progress
and not a release candidate.
