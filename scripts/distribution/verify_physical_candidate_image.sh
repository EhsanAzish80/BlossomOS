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
  usr/lib/blossom-os/blossom-shell-service
  usr/lib/blossom-os/blossom-model-gateway
  usr/lib/blossom-os/blossom-privileged-helper
  usr/lib/blossom-os/blossom-shell-ui
  usr/lib/blossom-os/blossom-desktop-launcher
  usr/local/bin/blossom-shell-recovery
  usr/local/bin/blossom-start-session
  usr/local/bin/blossom-desktop-probe
)

required_files=(
  usr/share/blossom-os/shell/shell.qml
  usr/share/blossom-os/shell/SecurityHost.qml
  usr/share/blossom-os/shell/wallpaper.svg
  usr/lib/systemd/user/blossom-desktop-shell.service
  usr/lib/systemd/user/blossom-shell-ui.service
  usr/lib/systemd/user/blossom-desktop-launcher.service
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

echo "Candidate live filesystem executable boundary passed."
