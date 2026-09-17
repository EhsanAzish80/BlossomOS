# Desktop Runtime Readiness

Status: blocked before execution  
Date: 2026-09-17  
Reviewed source: `983598b` plus the graphical-smoke contract in this change

## Preflight result

The Apple-silicon development host has `qemu-system-x86_64`, `qemu-img` and
UEFI-capable VM launch tooling. It does not have a local Qt 6 QML development
runtime, Quickshell, or a running Linux container/Wayland environment capable of
executing the current shell directly.

No ISO representing the reviewed desktop source exists. The three
`blossom-os-0.11.0-physical-candidate-x86_64.iso` files found under
`/private/tmp` predate the current desktop toolbar, installer identity and login
changes. They are stale evidence and must not be used to qualify this source.

The candidate builder was invoked normally and refused to start with exit code
1 because `distribution/DESKTOP_BUILD_LOCK` is present. No lock was bypassed,
no image was compiled and no physical disk was touched.

## Gates that remain runtime-only

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
