#!/bin/bash
# Test BlossomOS in QEMU VM

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/build/out"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log() {
    echo -e "${GREEN}[BlossomOS Test]${NC} $1"
}

# Find the ISO
ISO_FILE=$(ls "$BUILD_DIR"/*.iso 2>/dev/null | head -n 1)

if [ -z "$ISO_FILE" ]; then
    echo -e "${BLUE}No ISO found. Build options:${NC}"
    echo "1. Full Arch-based: sudo ./build/build-iso.sh"
    echo "2. Light Alpine-based: ./build/build-alpine.sh"
    exit 1
fi

log "Found ISO: $ISO_FILE"

# Check for QEMU
if ! command -v qemu-system-x86_64 &> /dev/null; then
    log "QEMU not found. Installing..."
    if [[ "$OSTYPE" == "darwin"* ]]; then
        brew install qemu
    elif command -v apt-get &> /dev/null; then
        sudo apt-get install -y qemu-system-x86
    else
        echo "Please install QEMU manually"
        exit 1
    fi
fi

# VM Configuration
RAM="4G"
CORES="2"
DISK_SIZE="20G"
DISK_IMG="$BUILD_DIR/blossomos-test.qcow2"

# Create disk if it doesn't exist
if [ ! -f "$DISK_IMG" ]; then
    log "Creating virtual disk ($DISK_SIZE)..."
    qemu-img create -f qcow2 "$DISK_IMG" "$DISK_SIZE"
fi

log "Starting BlossomOS VM..."
log "RAM: $RAM | Cores: $CORES"
log "Press Ctrl+Alt+G to release mouse"
log "Press Ctrl+Alt+F to toggle fullscreen"

# Launch QEMU with proper settings for GUI
qemu-system-x86_64 \
    -name "BlossomOS Test" \
    -machine q35,accel=hvf \
    -cpu host \
    -smp "$CORES" \
    -m "$RAM" \
    -cdrom "$ISO_FILE" \
    -drive file="$DISK_IMG",format=qcow2,if=virtio \
    -boot order=d \
    -vga virtio \
    -display cocoa,show-cursor=on \
    -device virtio-net-pci,netdev=net0 \
    -netdev user,id=net0,hostfwd=tcp::2222-:22 \
    -device qemu-xhci \
    -device usb-kbd \
    -device usb-tablet \
    -audiodev coreaudio,id=audio0 \
    -device intel-hda \
    -device hda-duplex,audiodev=audio0

log "VM session ended"
