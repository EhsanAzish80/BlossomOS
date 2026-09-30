# Blossom Desktop graphical smoke gate

Status: defined, not executed. The candidate build lock remains active.

This gate validates the packaged desktop in a nested disposable compositor
before any ISO is allowed. Source checks and a successful process start do not
close it.

## Required captures

Capture one lossless screenshot and the associated service log for each state:

1. Live Welcome over a visible wallpaper, top bar and dock.
2. Desktop after **Continue to desktop**, with Welcome absent and chrome intact.
3. Application launcher open from the top bar and from the dock.
4. Files and Terminal open together, with a discoverable close/switch path.
5. Graphical network editor showing disconnected or connected state.
6. Live installer terminal open, or a visible accessible launch failure.
7. Agent composer open without hiding or replacing ordinary applications.
8. One bounded request reaches the local model and produces an exact approval
   preview in the separate security window.
9. Centred restart confirmation and centred shutdown confirmation.
10. Installed-mode desktop with no Install action.
11. Installed second session with onboarding still dismissed.
12. Quick Settings with Wi-Fi/Ethernet, volume/mute, Bluetooth, notifications
    and battery/AC states visible without horizontal clipping.
13. Centred logout confirmation followed by the graphical login screen and a
    successful login with the installer-created account.

## Layout matrix

- 1280x720 at scale 1.
- 1920x1080 at scale 1.
- 2880x1800 at scale 2, evaluated as a 1440x900 logical desktop.

At every size the dock must fit, the welcome card must remain fully visible,
the top bar must reserve space, popup controls must remain reachable and the
background must never receive focus or an active-window border.

## Interaction assertions

- Mouse and keyboard can open and close every overlay.
- Escape dismisses launcher and system overlays without invoking an action.
- Volume down/up and mute change the displayed status, and unavailable audio
  remains explicit rather than disappearing.
- Wi-Fi and Ethernet are distinguished; an absent adapter and disconnected
  connection remain readable and Network Settings still opens.
- Bluetooth on, off and unavailable states are readable; Devices opens the
  graphical manager without granting shell or root authority.
- Do Not Disturb suppresses a test notification and toggling it again restores
  notification delivery.
- Super+Tab changes the active ordinary application.
- Super+Q closes only the active ordinary application.
- Dismissing Welcome never stops the desktop service.
- The live installer action is absent when `BLOSSOM_LIVE` is absent.
- Logout returns to ReGreet, a wrong password is rejected, and the selected
  installer-created account can start a fresh Blossom session.
- A rejected desktop launch produces an accessibility alert.
- An unavailable model produces a visible fail-closed message and no effect.
- The agent Send control cannot bypass the service-authored preview or the
  password-backed approval window.
- Approval remains in the standard Qt accessibility host and denial remains the
  safe default.

## Pass record

The gate record must bind the source commit, package hashes, compositor/runtime
versions, logical display geometry and screenshot hashes. Any crash, missing
surface, unreadable text, incorrect live/installed visibility or unexplained
black screen returns the work to source review. It does not authorize an ISO.
