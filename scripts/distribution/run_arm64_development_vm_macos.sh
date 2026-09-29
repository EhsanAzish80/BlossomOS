#!/usr/bin/env bash
set -euo pipefail

repo=$(cd "$(dirname "$0")/../.." && pwd -P)
image=${1:-"$repo/.local-arm64/image/blossom-os-arm64-development.qcow2"}
state=${BLOSSOM_ARM64_VM_STATE:-"$repo/.local-arm64/vm"}
resolution=${BLOSSOM_ARM64_VM_RESOLUTION:-1280x800}

case "$resolution" in
  1280x800|1440x900|1600x1000|1920x1080) ;;
  *)
    echo "error: unsupported VM resolution: $resolution" >&2
    echo "supported resolutions: 1280x800, 1440x900, 1600x1000, 1920x1080" >&2
    exit 2
    ;;
esac
xres=${resolution%x*}
yres=${resolution#*x}

if [[ $(uname -s) != Darwin || $(uname -m) != arm64 ]]; then
  echo "error: this launcher requires an Apple-silicon Mac" >&2
  exit 2
fi
for command in qemu-img qemu-system-aarch64; do
  command -v "$command" >/dev/null || { echo "error: missing command: $command" >&2; exit 2; }
done
[[ -f "$image" ]] || { echo "error: ARM64 image not found: $image" >&2; exit 2; }

mkdir -p "$state"
disk="$state/blossom-os-arm64-development.qcow2"
firmware=/opt/homebrew/share/qemu/edk2-aarch64-code.fd
[[ -f "$firmware" ]] || { echo "error: ARM64 UEFI firmware not found: $firmware" >&2; exit 2; }
if [[ ! -f "$disk" || "$image" -nt "$disk" ]]; then
  rm -f "$disk"
  qemu-img create -f qcow2 -F qcow2 -b "$image" "$disk"
fi

echo "Starting native ARM64 Blossom OS development VM at ${resolution} with Apple HVF acceleration."
exec qemu-system-aarch64 \
  -name "Blossom OS ARM64 Development" \
  -machine virt,accel=hvf,highmem=on \
  -cpu host \
  -smp 4 \
  -m 8192 \
  -drive "if=pflash,format=raw,readonly=on,file=$firmware" \
  -drive "if=none,format=qcow2,file=$disk,id=system" \
  -device virtio-blk-pci,drive=system \
  -device "virtio-gpu-pci,edid=on,xres=$xres,yres=$yres" \
  -device virtio-keyboard-pci \
  -device virtio-tablet-pci \
  -device virtio-net-pci,netdev=net0 \
  -netdev user,id=net0 \
  -device virtio-rng-pci \
  -audiodev coreaudio,id=audio0 \
  -device virtio-sound-pci,audiodev=audio0 \
  -display cocoa,gl=off,zoom-to-fit=off \
  -serial mon:stdio
