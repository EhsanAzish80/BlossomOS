# ADR-0037: Adopt labwc after the desktop trust prerequisites are met

**Status:** Accepted

**Date:** 2026-10-01

## Context

Blossom currently uses Hyprland. Its tiling-first window behavior, missing conventional title bars, and compositor-specific global-shortcut integration are a poor fit for people who expect a conventional desktop. Shell work was also beginning to depend on Hyprland-specific APIs while trusted approval identified the compositor by the literal executable path `/usr/bin/Hyprland`.

Phase 12 requires a conventional window manager and working foundations for locking, idle handling, display configuration, application launching, the command bar, and trusted approval. A disposable ARM64 overlay was therefore used for a labwc spike. The base image was not modified and no image or GitHub build ran.

## Decision

Blossom will migrate to labwc, but the production switch is gated on all of the following landing together:

1. Replace the compositor-name trusted-approval anchor with a uniquely verified, active, local graphical logind session anchor. The selected process must be bound by UID, PID, start time, session scope, and parentage and rechecked before authorization. Zero, remote-only, or multiple candidate sessions fail closed.
2. Replace Hyprland's global-shortcut object with a labwc key binding that invokes the shell's Quickshell IPC handler.
3. Add the broker-backed dock window list and qualify list, focus, minimize, and close behavior.
4. Ship the labwc session, autostart, environment import, Petal Openbox theme, portal selection, and lock/idle/display tools as packages with source checks.
5. Rerun trusted approval, exact-effect disclosure, app launch, command-bar focus, and password-focus gates on the resulting installed packages before an image gate.

Until those prerequisites pass, Hyprland remains the production compositor. This is a migration decision, not evidence that the current image already satisfies it.

## Spike environment

- Architecture: AArch64
- Disposable qcow2 overlay over the known-good local base image
- labwc 0.20.2 and wlroots 0.20
- Existing Blossom Quickshell shell and installed-package services
- No full image build and no GitHub-hosted build

The existing resolver change was qualified in the same VM session. The Linux-only `workspace_plan::tests` run passed 11 tests with 0 failures, including folder-authority retention, non-regular and symlink rejection, hard-link overlap rejection, and the 65-folder limit.

## Evidence

| Check | Result | Evidence and consequence |
| --- | --- | --- |
| Shell renders | Pass with a small adaptation | Quickshell reached `Configuration Loaded` under labwc after replacing `Quickshell.Hyprland.GlobalShortcut` with `Quickshell.Io.IpcHandler`. The production migration must carry that change and its source check. |
| Command-bar shortcut route | Pass | labwc `rc.xml` binds Super+Space to `quickshell ipc call commandbar toggle`; the live shell exported the `commandbar.toggle()` target and accepted the IPC call. A final keyboard/focus check remains part of the installed-package gate. |
| Session environment for launched apps | Pass | The user manager contained `WAYLAND_DISPLAY=wayland-0`, `XDG_CURRENT_DESKTOP=labwc:wlroots`, `XDG_SESSION_TYPE=wayland`, and the session bus address. A transient graphical service launched `foot` successfully in `app-graphical.slice`. |
| Conventional windows and Petal title bars | Partial | labwc loaded with a Petal Openbox theme and created a decorated application window. The spike used labwc's built-in button glyphs; final Petal button assets and a visual accessibility check remain required. |
| Foreign-toplevel protocol | Protocol pass; product gap | labwc advertised `zwlr_foreign_toplevel_manager_v1` version 3. The current dock has no toplevel-manager implementation, so list, focus, minimize, and close are not yet product-qualified. |
| Approval and password focus | Fail on current code | `trusted_approval.rs` requires the executable `/usr/bin/Hyprland`. Under labwc it necessarily fails closed before interactive authorization. Focus and raising cannot be qualified until the logind-session trust refactor lands. |
| Compositor-independent session identity | Feasible, not implemented | logind reported exactly one active, local Wayland user session. Its root-owned leader was greetd, whose direct child was the expected user's labwc process in the same `session-1.scope`. This supports a session-rooted design without trusting a compositor name. |
| Layer shell | Pass | labwc advertised `zwlr_layer_shell_v1` version 4 and the existing shell loaded. |
| Display configuration | Pass | labwc advertised `zwlr_output_manager_v1` version 4; `wlr-randr` enumerated and described the active QEMU display. |
| Idle detection | Protocol pass | labwc advertised `ext_idle_notifier_v1` version 2 and `swayidle` was installed for the spike. The end-to-end idle action should be repeated after the guest-agent harness is restored. |
| Screen locking | Protocol pass; interactive result unverified | labwc advertised `ext_session_lock_manager_v1` version 1 and `swaylock` was installed. Invoking the live lock probe stopped the guest-agent channel from returning output, so lock/unlock is not claimed as passed. It remains an explicit migration gate. |
| kanshi compatibility | Protocol pass | The required output-management protocol was advertised. A persistent multi-output kanshi profile was not exercised on the single-output VM. |
| Desktop portal | Partial | `xdg-desktop-portal-wlr` ran, but the Hyprland portal was still activatable in the overlay. The labwc package must select the wlr backend and omit or mask the Hyprland backend. |

## Trusted approval anchor

The logind session leader itself is root-owned greetd, so it is not the `unix-process` subject for the user's authentication. The production implementation will select the expected user's uniquely verified direct child of the root-owned logind session leader, bind it to the active local graphical session scope, and retain the existing PID-reuse defenses (UID and process start time). It must recheck the relationship immediately before asking polkit.

This removes the compositor executable name from the security decision while keeping the subject tied to a process whose placement is established by the root-owned login session. Tests must cover zero sessions, two graphical sessions, a remote-only session, changed parentage, changed scope, changed UID, and PID/start-time mismatch.

## Consequences

- Blossom gains conventional floating windows, movable title bars, and familiar window controls without version-coupled Hyprland plugins.
- The shell remains based on layer-shell and uses compositor-neutral Quickshell IPC for the command bar.
- The compositor migration cannot merge as a cosmetic-only change because trusted approval and focus behavior are security gates.
- The dock needs a real foreign-toplevel implementation before the migration is complete.
- Phase 12 begins with known protocol support, while interactive screen locking and multi-output kanshi behavior remain named qualification work rather than assumed support.
