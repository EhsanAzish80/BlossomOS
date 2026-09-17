# Desktop Platform Integration Gate

Status: source implemented; runtime evidence pending  
Build status: blocked by `distribution/DESKTOP_BUILD_LOCK`

## Implemented source boundary

The reviewed desktop source now includes:

- `xdg-desktop-portal` with the Hyprland backend for compositor integration;
- the GTK portal fallback for file chooser and general desktop portal support;
- PipeWire-compatible screen-sharing prerequisites through the existing audio
  stack and Hyprland portal;
- Mako as the session notification service with a Blossom-owned visual config;
- Hyprpolkitagent as the graphical authentication agent;
- Qt Wayland and XWayland compatibility packages;
- Grim/Slurp screenshot tooling and a bounded Print-key screenshot command;
- systemd-user startup and D-Bus activation environment import from the active
  Hyprland session.

The portal selection is explicit:

```ini
[preferred]
default=hyprland;gtk
org.freedesktop.impl.portal.FileChooser=gtk
```

Hyprland handles compositor-specific functions such as screen sharing. GTK is
the declared file-chooser fallback. The implementation does not depend on a
desktop-specific environment variable guessing a backend.

## Deliberately blocked lock/idle behavior

Automatic locking and idle suspend are **not enabled**. The current live and
physical-qualification `blossom` accounts are created with locked passwords.
Starting a password-authenticated lock screen in that state can make the
session impossible to unlock.

The correct prerequisite is a real installed-account provisioning flow that:

1. creates or adopts a user-selected account;
2. establishes an authentication method without shipping a default password;
3. validates PAM unlock from the installed system;
4. separates live-session behavior from installed-session behavior;
5. tests lock-before-suspend and recovery on physical hardware.

Do not work around this gate with an empty password, embedded password,
passwordless unlock, automatic root login, or a lock-screen bypass.

## Required runtime evidence

Before this gate is complete, a disposable installed VM must prove:

1. `xdg-desktop-portal`, `xdg-desktop-portal-hyprland`, Mako and
   Hyprpolkitagent are active in the user session.
2. Firefox and one sandboxed test application open a native file chooser and
   receive access only to the selected file.
3. A screen-sharing test produces a chooser and a PipeWire stream, and denial
   produces no stream.
4. Print creates one screenshot in the user's Pictures/Screenshots directory
   and emits a readable notification.
5. A GUI action requiring Polkit displays an authentication dialog and denial
   causes no mutation.
6. Notifications are readable at standard and 2x scale and do not cover the
   top bar or critical approval dialog.
7. XWayland and native Wayland applications both launch, focus, close and use
   clipboard/drag-and-drop normally.
8. All ordinary desktop functions continue when agent services are disabled.

These are runtime gates. Package presence or a green source check alone does
not satisfy them.
