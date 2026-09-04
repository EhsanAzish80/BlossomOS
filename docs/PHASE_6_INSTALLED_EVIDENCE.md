# Phase 6 installed shell evidence

Status: harness implemented; no passing installed run is claimed yet.

`.github/workflows/phase6-installed-evidence.yml` creates a disposable official
Arch x86-64 userspace and fails unless the installed Hyprland, Quickshell,
systemd, and dbus-broker versions exactly match the accepted lock. It builds the
feature-gated Rust session service and native QML plugin, installs the fixed
binary, user unit, D-Bus activation metadata, plugin module, and QML files under
their intended root-owned paths, and checks unit and ELF dependencies.

For compositor evidence without pretending the GitHub runner has desktop
hardware, Weston provides a headless parent Wayland display and the real pinned
Hyprland runs nested on it. Hyprland launches the real pinned Quickshell, which
loads the installed Blossom QML and native plugin. The QML performs a bounded
activity refresh, requiring the fixed D-Bus service to activate successfully.
The harness rejects missing QML modules, unavailable types, or root-component
creation failure.

This establishes Arch userspace ABI, packaging, activation, nested compositor,
and configuration-load evidence only after a passing run is recorded here. It
does not prove GPU/DRM behavior, physical displays or input devices, compositor
security, hardware, an Arch kernel, installer, ArchISO, upgrade/rollback,
distribution packaging, or release readiness. Approval focus, close,
keyboard-only, assistive-technology, and complete graphical execution evidence
remain separate gates until explicitly exercised.
