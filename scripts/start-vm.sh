#!/bin/bash
# Simple BlossomOS VM Launcher for Mac

cd "$(dirname "$0")/../build/out"

echo "🌸 Starting BlossomOS..."
echo "Press Ctrl+C here to stop the VM"
echo ""

qemu-system-x86_64 \
    -name "BlossomOS" \
    -m 4G \
    -smp 2 \
    -cdrom alpine-extended-3.19-x86_64.iso \
    -drive file=blossomos-disk.qcow2,format=qcow2 \
    -boot order=d \
    -vga std \
    -display cocoa \
    -netdev user,id=net0,hostfwd=tcp::8022-:22 \
    -device e1000,netdev=net0
