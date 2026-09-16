#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
usage: run_candidate_vm_macos.sh ISO [STATE_DIRECTORY]

Boot an x86_64 Blossom OS candidate under software emulation on an
Apple-silicon Mac. The ISO is read-only. VM firmware state and the disposable
test disk are stored below STATE_DIRECTORY (default: .local-phase11-vm).
EOF
}

if (($# < 1 || $# > 2)); then
  usage >&2
  exit 2
fi

iso=$(realpath "$1")
state=${2:-.local-phase11-vm}
mkdir -p "$state"
state=$(realpath "$state")

if [[ ! -f "$iso" || "$iso" != *.iso ]]; then
  echo "ISO must name a regular .iso file" >&2
  exit 2
fi
for command in qemu-img qemu-system-x86_64; do
  command -v "$command" >/dev/null || {
    echo "missing required command: $command" >&2
    exit 1
  }
done

qemu_binary=$(command -v qemu-system-x86_64)
qemu_prefix=$(cd "$(dirname "$qemu_binary")/.." && pwd)
qemu_share="$qemu_prefix/share/qemu"
firmware_code="$qemu_share/edk2-x86_64-code.fd"
firmware_vars_source="$qemu_share/edk2-i386-vars.fd"
firmware_vars="$state/edk2-vars.fd"
disk="$state/blossom-test.qcow2"

if [[ ! -f "$firmware_code" || ! -f "$firmware_vars_source" ]]; then
  echo "QEMU UEFI firmware was not found under $qemu_share" >&2
  exit 1
fi
if [[ ! -f "$firmware_vars" ]]; then
  cp "$firmware_vars_source" "$firmware_vars"
fi
if [[ ! -f "$disk" ]]; then
  qemu-img create -f qcow2 "$disk" 48G
fi

echo "Booting: $iso"
echo "Disposable VM disk: $disk"
echo "Apple silicon runs this x86_64 guest through software emulation; boot is slower than Intel hardware."

exec qemu-system-x86_64 \
  -name "Blossom OS candidate" \
  -machine q35,accel=tcg \
  -cpu max \
  -smp 2 \
  -m 4096 \
  -drive "if=pflash,format=raw,readonly=on,file=$firmware_code" \
  -drive "if=pflash,format=raw,file=$firmware_vars" \
  -drive "if=virtio,format=qcow2,file=$disk" \
  -drive "media=cdrom,readonly=on,file=$iso" \
  -device virtio-vga \
  -device qemu-xhci \
  -device usb-tablet \
  -nic user,model=virtio-net-pci \
  -display cocoa,gl=off
