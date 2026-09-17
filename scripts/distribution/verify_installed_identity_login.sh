#!/usr/bin/env bash
set -euo pipefail

if (($# != 1)); then
  echo "usage: verify_installed_identity_login.sh USERNAME" >&2
  exit 2
fi
user=$1
[[ $user =~ ^[a-z][a-z0-9_-]{0,30}$ ]] || {
  echo "invalid username" >&2
  exit 2
}

test "$(systemctl get-default)" = graphical.target
systemctl is-enabled --quiet greetd.service
systemctl is-active --quiet greetd.service
test ! -e /etc/systemd/system/getty@tty1.service.d/autologin.conf
test ! -e /etc/systemd/system/getty@tty1.service.d/override.conf

owner=$(python3 -c 'import json; print(json.load(open("/var/lib/blossom/installation/owner.json"))["username"])')
test "$owner" = "$user"
getent passwd "$user" >/dev/null
test "$(passwd -S "$user" | awk '{print $2}')" = P
test "$(find /home -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort)" = "$user"
test -f "/home/$user/.config/hypr/hyprland.conf"
test -f /usr/share/wayland-sessions/blossom.desktop
grep -Fq 'Exec=/usr/bin/start-hyprland' /usr/share/wayland-sessions/blossom.desktop
loginctl show-user "$user" -p State --value | grep -Eq 'active|online'
pgrep -u "$user" -x Hyprland >/dev/null

echo "Installed identity and graphical-login runtime gate passed for $user."
