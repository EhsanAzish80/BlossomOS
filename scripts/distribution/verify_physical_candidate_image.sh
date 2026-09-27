#!/usr/bin/env bash
set -euo pipefail

if (($# != 1)); then
  echo "usage: verify_physical_candidate_image.sh ISO" >&2
  exit 2
fi

iso=$(realpath "$1")
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

bsdtar -xf "$iso" -C "$work" blossom/x86_64/airootfs.sfs
squashfs="$work/blossom/x86_64/airootfs.sfs"
root="$work/root"

executables=(
  usr/bin/grim
  usr/bin/mako
  usr/bin/notify-send
  usr/lib/hyprpolkitagent/hyprpolkitagent
  usr/lib/xdg-desktop-portal
  usr/lib/xdg-desktop-portal-gtk
  usr/lib/xdg-desktop-portal-hyprland
  usr/lib/blossom-os/blossom-shell-service
  usr/lib/blossom-os/blossom-model-gateway
  usr/lib/blossom-os/blossom-privileged-helper
  usr/lib/blossom-os/blossom-shell-ui
  usr/lib/blossom-os/blossom-desktop-launcher
  usr/lib/blossom-os/blossom-installer
  usr/local/bin/blossom-shell-recovery
  usr/local/bin/blossom-start-session
  usr/local/bin/blossom-screenshot
  usr/local/bin/blossom-desktop-probe
  usr/local/bin/blossom-install-observe
  usr/local/libexec/blossom-graphical-install-backend
)

required_files=(
  usr/share/blossom-os/shell/shell.qml
  usr/share/blossom-os/shell/SecurityHost.qml
  usr/share/blossom-os/shell/wallpaper.svg
  usr/share/blossom-os/installer/Main.qml
  usr/lib/systemd/user/blossom-desktop-shell.service
  usr/lib/systemd/user/blossom-shell-ui.service
  usr/lib/systemd/user/blossom-desktop-launcher.service
  home/blossom/.config/mako/config
  home/blossom/.config/xdg-desktop-portal/hyprland-portals.conf
  root/blossom-rootfs.tar.zst
  root/blossom-rootfs.tar.zst.sha256
)

unsquashfs -quiet -d "$root" "$squashfs" \
  "${executables[@]}" \
  "${required_files[@]}"

for path in "${executables[@]}"; do
  test -f "$root/$path" || {
    echo "candidate is missing required runtime: /$path" >&2
    exit 1
  }
  test -x "$root/$path" || {
    echo "candidate runtime is not executable: /$path" >&2
    exit 1
  }
done

(
  cd "$root/root"
  printf '%s  %s\n' "$(cat blossom-rootfs.tar.zst.sha256)" blossom-rootfs.tar.zst \
    | sha256sum --check --status
) || {
  echo "candidate embedded rootfs digest does not match" >&2
  exit 1
}

for path in "${required_files[@]}"; do
  test -f "$root/$path" || {
    echo "candidate is missing required desktop content: /$path" >&2
    exit 1
  }
done

grep -Fq "Welcome to Blossom OS" "$root/usr/share/blossom-os/shell/shell.qml" || {
  echo "candidate is missing the visible welcome surface" >&2
  exit 1
}
for namespace in blossom-background blossom-top-bar blossom-dock blossom-welcome; do
  grep -Fq "WlrLayershell.namespace: \"$namespace\"" \
    "$root/usr/share/blossom-os/shell/shell.qml" || {
    echo "candidate is missing desktop layer: $namespace" >&2
    exit 1
  }
done
grep -Fq "ExecStart=/usr/bin/quickshell -p /usr/share/blossom-os/shell" \
  "$root/usr/lib/systemd/user/blossom-desktop-shell.service" || {
  echo "candidate desktop service does not launch the packaged shell" >&2
  exit 1
}
grep -Fq "default=hyprland;gtk" \
  "$root/home/blossom/.config/xdg-desktop-portal/hyprland-portals.conf" || {
  echo "candidate portal preference is missing the Hyprland and GTK backends" >&2
  exit 1
}

installed="$work/installed"
mkdir -p "$installed"
tar --zstd -xf "$root/root/blossom-rootfs.tar.zst" -C "$installed" \
  ./etc/greetd/config.toml \
  ./etc/greetd/regreet.toml \
  ./etc/greetd/hyprland.conf \
  ./etc/systemd/system/default.target \
  ./etc/systemd/system/display-manager.service \
  ./usr/share/wayland-sessions/blossom.desktop
grep -Fq 'command = "Hyprland --config /etc/greetd/hyprland.conf"' \
  "$installed/etc/greetd/config.toml" || {
  echo "installed rootfs does not launch the graphical greeter" >&2
  exit 1
}
grep -Fq 'Exec=/usr/bin/start-hyprland' \
  "$installed/usr/share/wayland-sessions/blossom.desktop" || {
  echo "installed rootfs does not register the Blossom Wayland session" >&2
  exit 1
}
if tar --zstd -tf "$root/root/blossom-rootfs.tar.zst" | grep -Eq 'getty@tty1.*autologin|home/blossom/'; then
  echo "installed rootfs retained a fixed user or console autologin" >&2
  exit 1
fi

echo "Candidate live and installed filesystem boundaries passed."
