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
7. Agent activity open without hiding or replacing ordinary applications.
8. Centred restart confirmation and centred shutdown confirmation.
9. Installed-mode desktop with no Install action.
10. Installed second session with onboarding still dismissed.

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
- Super+Tab changes the active ordinary application.
- Super+Q closes only the active ordinary application.
- Dismissing Welcome never stops the desktop service.
- The live installer action is absent when `BLOSSOM_LIVE` is absent.
- A rejected desktop launch produces an accessibility alert.
- Approval remains in the standard Qt accessibility host and denial remains the
  safe default.

## Pass record

The gate record must bind the source commit, package hashes, compositor/runtime
versions, logical display geometry and screenshot hashes. Any crash, missing
surface, unreadable text, incorrect live/installed visibility or unexplained
black screen returns the work to source review. It does not authorize an ISO.
