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

unsquashfs -quiet -d "$root" "$squashfs" \
  "${executables[@]}" \
  usr/share/blossom-os/shell/shell.qml

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

grep -Fq "Welcome to Blossom OS" "$root/usr/share/blossom-os/shell/shell.qml" || {
  echo "candidate is missing the visible welcome surface" >&2
  exit 1
}

echo "Candidate live filesystem executable boundary passed."
