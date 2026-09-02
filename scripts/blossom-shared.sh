#!/bin/bash
# BlossomOS with Shared Folder

cd "$(dirname "$0")/../build/out"

# Create shared directory
mkdir -p ../shared
cp ../../scripts/fix-all.sh ../shared/ 2>/dev/null
cp ../../ai-core/blossom-ai.py ../shared/ 2>/dev/null

echo "🌸 Starting BlossomOS with shared folder..."
echo ""
echo "Shared folder: /mnt/shared (inside VM)"
echo "Access your Mac files from the VM!"
echo ""

qemu-system-x86_64 \
    -name "BlossomOS" \
    -m 4G \
    -smp 2 \
    -drive file=blossomos-disk.qcow2,format=qcow2 \
    -boot c \
    -vga std \
    -display cocoa,show-cursor=on \
    -netdev user,id=net0,hostfwd=tcp::2222-:22 \
    -device e1000,netdev=net0 \
    -virtfs local,path=../shared,mount_tag=host0,security_model=passthrough,id=host0 \
    > /dev/null 2>&1 &

echo "✅ BlossomOS started with shared folder!"
echo ""
echo "In the VM, mount shared folder:"
echo "  sudo mkdir -p /mnt/shared"
echo "  sudo mount -t 9p -o trans=virtio,version=9p2000.L host0 /mnt/shared"
