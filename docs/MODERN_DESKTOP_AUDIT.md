# Modern Desktop Capability Audit

Status: product and architecture audit; no implementation or release claim  
Date: 2026-09-17  
Applies to: the Desktop Foundation source branch while `distribution/DESKTOP_BUILD_LOCK` remains active

## Outcome

The current source is a credible **desktop foundation**, not yet a complete
daily-driver desktop. It has a branded session, application launcher, ordinary
files/browser/editor/terminal entry points, network and audio entry points,
onboarding, bounded agent activity, explicit privileged approval, and guarded
installation. The largest remaining product gaps are software discovery,
complete settings, session ergonomics, notifications and portals, accounts and
privacy controls, updates/recovery UI, and user-facing agent preferences.

Blossom should remain both:

1. a normal personal computer whose core desktop does not depend on an agent;
2. an agent-native computer whose local assistant is optional, inspectable,
   bounded by capabilities, and unable to silently acquire administrator power.

## Repository findings

### Present

- Hyprland/Wayland session with Blossom-owned background, top bar, dock,
  launcher, onboarding and system actions.
- Firefox, Thunar, Mousepad, Foot, NetworkManager editor and PulseAudio volume
  control in the candidate package set.
- Separate user services for desktop surfaces, security approval and the
  desktop launcher.
- Network and battery status providers with explicit unavailable states.
- Signed update verification, staged A/B-like lifecycle state, rollback and
  recovery logic at the evidence/core level.
- Local model gateway boundaries, explicit approval, durable-memory limits and
  audit records.

### Missing or incomplete

- No graphical software catalog or sandboxed application installation flow.
- No desktop portal backend, permission store UI, screen sharing, screenshots,
  native file chooser integration or application permission review.
- No unified Blossom Settings application.
- No end-user System Update, rollback or recovery interface wired to the signed
  lifecycle core.
- No notification center, do-not-disturb control or persistent notification
  history.
- No lock screen, idle policy, suspend controls or polished login/user switcher.
- No complete display, appearance, keyboard, pointer, touchpad, Bluetooth,
  printer, locale, time-zone, accessibility, default-app or storage settings.
- No agent preferences surface for models, memory, permissions, network use,
  resource budgets, startup behavior, activity history or reset/export.
- No user-facing backup/restore workflow.
- No tested application compatibility contract for Flatpak, XWayland, portals,
  drag-and-drop, clipboard, printing, cameras, microphones or removable media.

## Recommended product architecture

### 1. Blossom Software

Use a consumer-facing **Software** application for sandboxed desktop apps.

- Primary application source: Flatpak.
- Initial catalog: an explicitly configured and user-visible remote, with its
  trust/source explained during first use rather than silently added.
- Required platform pieces: `flatpak`, `xdg-desktop-portal`, a Hyprland portal
  backend, a general file/dialog portal backend where required, AppStream data,
  and a permissions-management UI.
- Show source, sandbox permissions, download size, installed size, license,
  update availability and whether an application is verified by its catalog.
- Support install, remove, update, launch, permission review and data removal.
- Never represent a third-party catalog as the “Blossom App Store.” Use
  “Software” and name the actual source.

Do **not** make raw Pacman or AUR packages the ordinary graphical app-store
path. They modify the base operating system, can execute package scripts with
system authority, and do not match the app-sandbox boundary. Pacman remains an
advanced terminal/admin tool. AUR support is out of scope for the first public
desktop release.

### 2. Blossom System Update

Keep operating-system updates separate from application updates.

- Consume only Blossom release metadata and payloads that pass the existing
  signature, identity, size, digest, expiry and anti-downgrade checks.
- Present current version/channel, last checked time, download size, release
  notes, restart requirement and rollback availability.
- Stage into the inactive system slot, verify it, reboot into it, and retain a
  visible rollback path.
- Never run unattended system upgrades while the device is low on power,
  storage is insufficient, or recovery evidence is unhealthy.
- Application updates may be automatic by policy; OS updates should initially
  require a clear user decision and provide a scheduled-restart option.
- Firmware updates belong in a separately labeled Hardware/Firmware section and
  require explicit device compatibility and power checks.

### 3. Blossom Settings

Create one native settings application with searchable categories:

| Category | First useful scope |
| --- | --- |
| Appearance | wallpaper, light/dark, accent, text/UI scale, cursor, reduced motion |
| Displays | arrangement, resolution, refresh rate, scale, orientation, night light |
| Network | Wi-Fi/Ethernet/VPN state, connection editor, metered/offline behavior |
| Bluetooth | discover, pair, disconnect, remove, adapter state and recovery |
| Sound | input/output device, volume, mute, microphone level and test |
| Power | battery health/status, suspend/idle, lid behavior, performance profile |
| Keyboard | layout, shortcuts, repeat rate, compose key and shortcut conflicts |
| Mouse & Touchpad | speed, natural scroll, tap-to-click, scroll and gestures |
| Notifications | per-app permissions, do-not-disturb and history controls |
| Applications | defaults, startup apps, file associations and sandbox permissions |
| Privacy & Security | camera, microphone, location, files, screen capture, USB and audit history |
| Users | user identity, avatar, password and administrator role boundaries |
| Region & Language | locale, formats, keyboard layouts, time zone and clock |
| Accessibility | large text, contrast, screen reader, keyboard navigation, reduced motion |
| Printers | discovery, queue state, defaults and test page |
| Storage | capacity, application data, caches, removable media and cleanup previews |
| Backup & Recovery | backup target/status, restore, recovery media and rollback |
| System | device information, logs/export, updates, firmware and support bundle |
| Agent | the preferences specified below |

Each settings page needs unavailable/unsupported states. It must not merely
hide missing hardware or silently fail.

### 4. Agent preferences

The agent page should expose meaningful behavior without weakening the security
model:

- Master state: Off, available on demand, or start with session.
- Active local model/provider, readiness, model size, storage location and
  remove/download action.
- Resource budgets: maximum memory, CPU priority, battery behavior and pause on
  low power.
- Network policy: offline only by default; separately authorize model download,
  web access and other network capabilities.
- Memory: off/session/durable, categories allowed, retention period, review,
  delete, export and reset.
- Capability permissions: Ask every time, allow for this session, or deny.
  Persistent allow should be offered only to capabilities proven safe for that
  scope; privileged mutation must retain an approval ceremony.
- Context sources: clipboard, selected files, notifications, active application,
  calendar and other future sources default off and explain lifetime/scope.
- Behavior: response style, notification level, confirmation verbosity and
  whether suggested actions may be prepared but not executed.
- Activity and audit: human-readable requests, approvals, denials, outcomes,
  model identity and timestamps, with redaction and export controls.
- Emergency controls: stop current work, disable all agent services, clear
  session context, reset preferences and remove local models.

There must be no “always allow everything,” generic shell, unrestricted root,
or silent privilege escalation preference.

### 5. Desktop essentials

Before expanding agent features, complete ordinary-computer behavior:

- predictable floating/tiled window controls and discoverable close/minimize/
  maximize actions;
- task switching, active/running indicators, multiple workspaces and overview;
- notification toasts plus a notification center;
- lock, suspend, logout, restart and shutdown with centered confirmations;
- clipboard, drag-and-drop, file associations and default applications;
- screen capture and screen sharing through portals;
- removable-drive mount/eject feedback;
- Bluetooth, camera, microphone and printer workflows;
- searchable help, keyboard-shortcut reference and a first-run tour;
- crash recovery that identifies which component failed without taking down the
  ordinary desktop.

## Delivery order

### Foundation Gate A — platform integration

1. Add and validate XDG desktop portals for Hyprland plus file chooser support.
2. Add notifications, policy authentication agent, lock/idle and session
   controls.
3. Test clipboard, drag-and-drop, file chooser, screenshots, screen sharing,
   audio input/output and removable media in a disposable installed VM.

### Foundation Gate B — settings

1. Build the Blossom Settings shell and schema-backed preference service.
2. Deliver Appearance, Displays, Network, Sound, Power, Keyboard, Mouse &
   Touchpad, Applications, Privacy, Accessibility and System first.
3. Add hardware-specific pages only with explicit unavailable states and
   physical evidence.

### Foundation Gate C — software and updates

1. Prove Flatpak/portal installation and uninstall in a disposable VM.
2. Add Software with one disclosed remote and permission review.
3. Connect System Update to signed Blossom lifecycle metadata and rollback.
4. Add firmware only after supported-device and recovery testing.

### Foundation Gate D — agent preferences

1. Define a versioned settings schema with safe defaults and migration.
2. Implement model/resource/network/memory/context/capability settings.
3. Add complete audit, export, reset and emergency-stop flows.
4. Test that the entire ordinary desktop continues to work with every agent
   service disabled or failed.

### Release Gate

Do not call the result a modern daily-driver candidate until all four foundation
gates pass keyboard-only and HiDPI graphical smoke tests in both live and
installed modes, followed by one successful generic VM install/boot/update/
rollback cycle and bounded physical-hardware qualification.

## Decisions required before implementation

1. Choose the first Software frontend: integrate an established frontend for
   speed, or build a Blossom-native Flatpak-only frontend for visual and policy
   control. A short disposable prototype should decide this.
2. Decide whether the first public release configures a third-party Flatpak
   remote during onboarding or asks the user to add it on first Software launch.
3. Define the signed Blossom update service/repository and release-channel
   policy; the evidence lifecycle code is not itself a production update
   service.
4. Choose the settings persistence boundary: user preferences, system policy
   and agent security decisions must have separate owners and permissions.
5. Define exactly which agent permissions may persist. Privileged mutations
   should remain approve-once until evidence justifies anything broader.

## Explicit non-goals for the next candidate

- AUR helper or arbitrary package-script execution.
- Cloud account requirement, telemetry, advertisements or remote agent by
  default.
- A custom replacement for mature Linux networking, audio, printing or portal
  services.
- Automatic destructive cleanup or silent system repair.
- Opening the ISO build lock before the platform, settings and graphical smoke
  contracts are implemented and reviewed.
