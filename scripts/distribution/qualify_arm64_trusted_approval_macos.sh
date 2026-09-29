#!/usr/bin/env bash
set -euo pipefail

repo=$(cd "$(dirname "$0")/../.." && pwd -P)
image=${1:-"$repo/.local-arm64-trusted-approval/image/blossom-os-arm64-development.qcow2"}
state=${BLOSSOM_ARM64_TRUSTED_APPROVAL_STATE:-"$repo/.local-arm64-trusted-approval/qualification"}

if [[ $(uname -s) != Darwin || $(uname -m) != arm64 ]]; then
  echo "error: this qualification requires an Apple-silicon Mac" >&2
  exit 2
fi
for command in qemu-img qemu-system-aarch64; do
  command -v "$command" >/dev/null || { echo "error: missing command: $command" >&2; exit 2; }
done
[[ -f $image ]] || { echo "error: ARM64 image not found: $image" >&2; exit 2; }
image=$(cd "$(dirname "$image")" && pwd -P)/$(basename "$image")

mkdir -p "$state"
disk="$state/qualification.qcow2"
result="$state/result.log"
serial="$state/serial.log"
firmware=/opt/homebrew/share/qemu/edk2-aarch64-code.fd
[[ -f $firmware ]] || { echo "error: ARM64 UEFI firmware not found: $firmware" >&2; exit 2; }
rm -f "$disk" "$result" "$serial"
qemu-img create -f qcow2 -F qcow2 -b "$image" "$disk" >/dev/null

qemu-system-aarch64 \
  -name "Blossom OS Trusted Approval Qualification" \
  -machine virt,accel=hvf,highmem=on \
  -cpu host -smp 4 -m 8192 \
  -drive "if=pflash,format=raw,readonly=on,file=$firmware" \
  -drive "if=none,format=qcow2,file=$disk,id=system" \
  -device virtio-blk-pci,drive=system \
  -device virtio-gpu-pci -device virtio-keyboard-pci -device virtio-tablet-pci \
  -device virtio-net-pci,netdev=net0 -netdev user,id=net0 \
  -device virtio-rng-pci -device virtio-serial-pci \
  -chardev null,id=qga \
  -device virtserialport,chardev=qga,name=org.qemu.guest_agent.0 \
  -chardev "file,id=qualification,path=$result" \
  -device virtserialport,chardev=qualification,name=org.blossomos.qualification \
  -display none -monitor none -serial "file:$serial" &
pid=$!

cleanup() {
  if kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

deadline=$((SECONDS + 240))
while kill -0 "$pid" 2>/dev/null && ((SECONDS < deadline)); do sleep 2; done
if kill -0 "$pid" 2>/dev/null; then
  echo "error: trusted approval qualification timed out" >&2
  exit 1
fi
wait "$pid" || true

cat "$result"
grep -q '^BLOSSOM_ARM64_TRUSTED_APPROVAL_READY self_approval=denied missing_agent=denied effects=0 executor_starts=0 bypass=absent ' "$result" || {
  echo "error: ARM64 trusted approval qualification failed; serial log: $serial" >&2
  exit 1
}
echo "ARM64 trusted approval automation passed."
