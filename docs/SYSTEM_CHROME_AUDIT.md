# System Chrome Audit

Status: source controls implemented; runtime evidence pending  
Date: 2026-09-17

## Outcome

The Blossom top bar now exposes the minimum ordinary-computer controls directly
instead of hiding them in the application launcher:

- Wi-Fi and Ethernet status with access to NetworkManager configuration;
- sound details, volume down/up and mute through PipeWire;
- Bluetooth device management through Blueman;
- battery percentage/state or explicit AC/unavailable state;
- clock, application launcher and system menu;
- notification do-not-disturb control backed by a declared Mako mode;
- logout, restart and shutdown with confirmation.

Every action is a fixed entry in the root-owned desktop launcher allowlist. The
QML shell does not receive a generic command, shell string, root authority or a
device mutation API. Unsupported/unavailable battery and network states remain
visible rather than silently disappearing.

## Known release gates

- Connectivity exposes NetworkManager's combined connectivity class and whether
  the active link is Wi-Fi or Ethernet; it does not display the SSID, interface
  name or VPN identity because those need a separately reviewed privacy surface.
- Sound shows current volume/mute and provides fixed `wpctl` actions, but selected
  input/output device names are not yet projected back into the shell.
- Bluetooth shows adapter power and opens a full device manager; individual
  paired-device state is intentionally left to that manager for now.
- Mako supplies visible notifications and do-not-disturb, but not a persistent
  notification-history center.
- Lock and suspend remain disabled until the new installed password and PAM
  unlock path pass disposable-VM testing.

These gaps do not justify another physical image yet. The next disposable VM
must prove the visible controls, volume/mute changes, Wi-Fi versus Ethernet,
Bluetooth adapter absence, device error states, logout/relogin and
authentication. Persistent notification history remains a requirement before
calling Blossom a complete modern desktop.
