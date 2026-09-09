# Beta-candidate limitations and supported target

Blossom OS remains pre-release research software. The only Phase 10 evidence
target is an x86-64 UEFI virtual machine matching the closed distribution
manifest. Evidence on that target does not make a physical computer supported.

## Supported for evidence

- Architecture: x86-64.
- Firmware and boot: UEFI virtual machine.
- Installation: fresh blank disposable disk using the reviewed image workflow.
- Updates: signed offline A/B evidence lifecycle only.
- Network: disabled for installation and lifecycle evidence.

## Not supported

- Daily-use installations or preservation of an existing operating system.
- Physical laptops, desktops, GPUs, Wi-Fi, Bluetooth, cameras, suspend, battery,
  accessibility hardware, or peripherals.
- Legacy BIOS, Secure Boot, disk encryption, dual boot, migration, ARM, or other
  architectures.
- Online or unattended updates, remote administration, telemetry, or production
  model downloads.
- Security updates for any published version; no supported release exists yet.

The Linux self-hosted machine is trusted build and test infrastructure. Its CPU
architecture and graphics device do not constitute hardware compatibility
evidence for Blossom OS.
