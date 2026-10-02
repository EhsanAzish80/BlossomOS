# Petal component board

Petal defines the ordinary, non-security presentation components used by the
Blossom shell. Security approval retains its separate fixed broker-controlled
surface and does not inherit Petal styling.

## Compact context menus

- Context menus are small, content-sized lists of direct user actions. They
  open by right-click, the Menu key, or Shift+F10; Up and Down move between
  rows, Enter activates, and Escape closes. Every row exposes an AT-SPI name,
  role, and keyboard focus state.
- A context menu closes as soon as approval begins. It must never cover the
  approval window or the broker-backed approval-pending cue.
- Context-menu actions are direct user actions only. Menu components must not
  call agent request methods. Agent work starts through the command bar and any
  effect still goes through the separate exact-effect approval ceremony.
