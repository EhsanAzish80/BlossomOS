# Desktop Runtime Readiness

Status: Linux compile qualified; graphical runtime blocked
Date: 2026-09-17
Reviewed source: `e2a9617`

## Preflight result

The Apple-silicon development host has `qemu-system-x86_64`, `qemu-img` and
UEFI-capable VM launch tooling. A disposable x86-64 Arch Linux container was
created under software emulation and received the committed source through a
read-only archive stream.

The following Linux Qt 6 components configured and compiled successfully with
their repository `-Wall -Wextra -Wpedantic -Werror` policies:

- `system/desktop-launcher`;
- `system/shell/client-plugin`, including the broker, QML module and UI host;
- `system/installer`.

The compile environment did not contain Quickshell or a nested Wayland
compositor, so this proves Linux build correctness but not the rendered shell.

No ISO representing the reviewed desktop source exists. The three
`blossom-os-0.11.0-physical-candidate-x86_64.iso` files found under
`/private/tmp` predate the current desktop toolbar, installer identity and login
changes. They are stale evidence and must not be used to qualify this source.

The candidate builder still refuses its default physical mode while
`distribution/DESKTOP_BUILD_LOCK` is present. The reviewed exception permits
only `BLOSSOM_CANDIDATE_MODE=vm-qualification`, producing disposable media for
this graphical gate. No physical disk may be touched.

## Gates that remain graphical-runtime-only

- 1280x720, 1920x1080 and 2x HiDPI visual captures;
- top-bar and popup fit, keyboard navigation and Escape dismissal;
- real NetworkManager Wi-Fi/Ethernet and unavailable-device states;
- PipeWire volume/mute behavior and device selection;
- Bluetooth adapter on/off/absent behavior and Blueman launch;
- Mako notification and Do Not Disturb behavior;
- logout to ReGreet, wrong-password rejection and successful account login;
- portal, Polkit, screenshot, Wayland/XWayland and agent-disabled behavior.

These checks require one image built from the reviewed commit in a disposable
x86-64 UEFI VM. Source tests and older media cannot satisfy them.
