# Phase 11 desktop usability gate

Status: implemented in source; runtime VM evidence required before rebuilding
physical media.

The Phase 11 candidate is no longer allowed to qualify with only a compositor,
pointer, diagnostic control, and terminal. The same reviewed desktop package is
used by the live and installed systems and must provide a coherent minimum
Linux workstation surface:

- first-run explanation that distinguishes live use from installation;
- an application launcher with Files, Firefox, text editor, terminal, network
  configuration, and audio settings;
- guarded installation entry only in the live environment;
- visible network, battery or AC, clock, agent state, and optional activity;
- confirmed restart and shutdown actions;
- keyboard access to terminal, files, browser, close, fullscreen, floating
  state, five workspaces, and moving windows between those workspaces; and
- readable 1280x720 layout, keyboard focus, large-text resilience, and clear
  unavailable/error states.

The shell UI retains fixed code-owned actions. It cannot accept an executable,
argument, path, or command from QML. Because the UI service is deliberately
sandboxed, fixed desktop programs are dispatched through the active Hyprland
session rather than being launched as children that inherit the UI sandbox.
The guarded installer remains a distinct Polkit-mediated action.

## Required VM evidence before another ISO is written to physical media

1. The live session reaches the welcome surface without manual commands.
2. Applications opens and closes with keyboard and pointer input.
3. Files, browser, editor, terminal, network, and audio actions each launch.
4. Network, power, clock, agent status, and activity surfaces remain readable.
5. The installer explains the target and performs no write without its separate
   exact confirmation.
6. Window close/fullscreen and workspace shortcuts operate as documented.
7. Restart and shutdown require confirmation and reach the requested state.
8. A failed shell opens the recovery surface instead of leaving an unexplained
   black screen.

Passing source tests is not runtime evidence. The next candidate is built once,
checksum-verified, booted first with `run_candidate_vm_macos.sh`, and written to
external media only after this matrix is observed.
