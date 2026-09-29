# Installer, Identity and Login Gate

Status: source foundation only; not build-authorized or release-qualified  
Date: 2026-09-17

## Product decision

The live image enters a temporary Blossom desktop without credentials. An
installed system never inherits that account and never autologs in. Installation
is a graphical, keyboard-accessible onboarding journey whose base path works
offline. The installed owner chooses their identity and password before the
first write begins.

## Graphical journey

Welcome -> language and keyboard -> network or offline -> hardware readiness ->
target disk -> owner account -> time zone -> review -> exact erase confirmation
-> readable progress -> success and restart.

Every step has Back and Quit, explicit unavailable/error states, keyboard focus,
large-text behavior and a stable summary. Disk identity remains visible from
selection through confirmation. The password fields support show/hide locally,
but their contents never enter logs, claim evidence, command-line arguments,
crash reports or the persisted installation profile.

## Account boundary

- There is no default installed username or password.
- Usernames are lowercase, validated and checked against reserved system names.
- Passwords are confirmed and passed to `chpasswd --root` over standard input.
- Only non-secret locale, time-zone, keyboard, name, username, administrator and
  agent-default data may be persisted.
- The first owner may receive the `wheel` role, but ordinary desktop use does not
  require administrator authority.
- Agent mode defaults to Off; On demand is the only other initial choice.

## Login boundary

The installed system uses greetd with ReGreet and a Blossom session entry. Its
background, typography and cursor match the desktop. The greeter provides user
selection, password entry, keyboard layout, accessibility, restart and shutdown.
It starts the session as the authenticated user through PAM. Console autologin is
permitted only on the temporary live image.

Screen locking and idle suspension remain disabled until a disposable installed
test proves PAM unlock, logout/login, restart persistence and recovery access.

## Activation checklist

- [x] Replace the terminal launcher with the packaged graphical installer.
- [x] Connect the wizard to the guarded target observer without adding write authority.
- [x] Pass a short-lived secret channel separately from public installation state.
- [x] Invoke owner provisioning only after rootfs extraction and before unmount.
- [x] Remove the build-time installed `blossom` account and installed getty autologin.
- [x] Package and enable greetd/ReGreet only in the installed rootfs.
- [x] Preserve live-session autologin and hide the installer after installation.
- [x] Test invalid/reserved names, password mismatch and secret non-retention.
- [ ] Test login failure/success, logout/login, reboot, power controls and recovery.
- [ ] Test keyboard-only, large text and 1x/2x display scale.

After a disposable installed boot, run
`sudo /opt/blossom/scripts/distribution/verify_installed_identity_login.sh USERNAME`
to collect the non-secret identity/session portion of the runtime gate. Successful
password rejection/acceptance, logout/relogin and accessibility interactions
remain human-observed graphical evidence and cannot be replaced by source tests.

The candidate build lock remains active until these items and the Desktop
Foundation gate are reviewed. These source files do not claim a working installer
or login screen in an image.
