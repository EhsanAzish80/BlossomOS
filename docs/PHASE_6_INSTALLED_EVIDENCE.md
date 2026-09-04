# Phase 6 installed shell evidence

Status: harness implemented; no passing installed run is claimed yet.

`.github/workflows/phase6-installed-evidence.yml` is a manually dispatched,
owner-provided GPU-runner gate. It creates a disposable official Arch x86-64
userspace and fails unless the installed Hyprland, Quickshell, systemd, and
dbus-broker versions exactly match the accepted production lock. Evidence-only
parent compositor versions are kept in a separate lock and do not widen that
closed production package set. The workflow builds the feature-gated Rust
session service and native QML plugin, installs the fixed binary, user unit,
D-Bus activation metadata, plugin module, and QML files under their intended
root-owned paths, and checks unit and ELF dependencies.

The runner must expose `/dev/dri` to the job container. Weston provides the
outer headless display, current Cage/wlroots provides the DMA-BUF-capable nested
Wayland parent required by Aquamarine, and the real pinned Hyprland runs inside
it. Hyprland launches the real pinned Quickshell, which loads the installed
Blossom QML and native plugin. The QML performs a bounded activity refresh,
requiring the fixed D-Bus service to activate successfully. The harness rejects
missing QML modules, unavailable types, or root-component creation failure.

Hosted-runner investigation is recorded by failed run `33890423701`: Cage and
Hyprland were real installed binaries, but the hosted container had no DRM
render node. Cage therefore used its Pixman/SHM allocator, while Aquamarine
requires `zwp_linux_dmabuf_v1` and refused to start. That run is diagnostic
evidence only and is not a passing installed-runtime claim.

Owner action: register a trusted, ephemeral GitHub Actions runner with labels
`self-hosted`, `linux`, `x64`, and `blossom-gpu`, with Docker permission to pass
`/dev/dri` into the container, then manually dispatch the workflow. Do not use a
general-purpose personal workstation or a runner holding unrelated secrets.

This will establish Arch userspace ABI, packaging, activation, nested
compositor, and configuration-load evidence only after a passing run is
recorded here. It will exercise a DRM render node but will not prove broad GPU
compatibility, physical displays or input devices, compositor security, an Arch
kernel, installer, ArchISO, upgrade/rollback, distribution packaging, or release
readiness. Approval focus, close,
keyboard-only, assistive-technology, and complete graphical execution evidence
remain separate gates until explicitly exercised.
