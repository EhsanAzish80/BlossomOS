if [[ -z ${WAYLAND_DISPLAY:-} && ${XDG_VTNR:-0} == 1 ]]; then
if grep -qw 'blossom.recovery=1' /proc/cmdline; then
  printf '\nBlossom OS recovery console\nRun blossom-physical-install only when you intend to install.\n\n'
else
  exec start-hyprland
fi
fi
