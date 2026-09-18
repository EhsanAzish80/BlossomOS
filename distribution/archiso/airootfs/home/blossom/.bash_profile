#!/usr/bin/env bash

if [[ -z ${WAYLAND_DISPLAY:-} && ${XDG_VTNR:-0} == 1 ]]; then
if grep -qw 'blossom.recovery=1' /proc/cmdline; then
  printf '\nBlossom OS recovery console\nRun blossom-physical-install only when you intend to install.\n\n'
else
  session_log="${XDG_STATE_HOME:-$HOME/.local/state}/blossom/session.log"
  mkdir -p "${session_log%/*}"
  if ! start-hyprland >"$session_log" 2>&1; then
    printf '\nBlossom OS could not start the desktop.\nDiagnostics were saved to %s\n' "$session_log"
  fi
fi
fi
