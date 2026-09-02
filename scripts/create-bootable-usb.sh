#!/bin/bash
# Create bootable USB for BlossomOS (Mac compatible)

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() {
    echo -e "${GREEN}[USB Creator]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Find ISO
BUILD_DIR="$(cd "$(dirname "$0")/../build/out" && pwd)"
ISO_FILE=$(ls "$BUILD_DIR"/*.iso 2>/dev/null | head -n 1)

if [ -z "$ISO_FILE" ]; then
    error "No ISO found. Build it first with: sudo ./build/build-iso.sh"
fi

log "Found ISO: $ISO_FILE"
log "ISO size: $(du -h "$ISO_FILE" | cut -f1)"

# List available disks
echo ""
log "Available disks:"

if [[ "$OSTYPE" == "darwin"* ]]; then
    diskutil list
    echo ""
    read -p "Enter disk identifier (e.g., disk2): " DISK
    DISK_PATH="/dev/$DISK"
    
    # Verify it's not the system disk
    if [[ "$DISK" == "disk0" ]]; then
        error "Cannot use disk0 (system disk)"
    fi
    
    warn "This will ERASE all data on $DISK"
    read -p "Are you sure? Type 'YES' to confirm: " confirm
    
    if [ "$confirm" != "YES" ]; then
        log "Cancelled"
        exit 0
    fi
    
    log "Unmounting disk..."
    diskutil unmountDisk "$DISK_PATH" || true
    
    log "Writing ISO to USB (this may take 5-10 minutes)..."
    sudo dd if="$ISO_FILE" of="$DISK_PATH" bs=4m status=progress
    
    log "Syncing..."
    sync
    
    log "Ejecting..."
    diskutil eject "$DISK_PATH"
    
else
    # Linux
    lsblk -p
    echo ""
    read -p "Enter device path (e.g., /dev/sdb): " DISK_PATH
    
    warn "This will ERASE all data on $DISK_PATH"
    read -p "Are you sure? Type 'YES' to confirm: " confirm
    
    if [ "$confirm" != "YES" ]; then
        log "Cancelled"
        exit 0
    fi
    
    log "Unmounting partitions..."
    sudo umount ${DISK_PATH}* 2>/dev/null || true
    
    log "Writing ISO to USB..."
    sudo dd if="$ISO_FILE" of="$DISK_PATH" bs=4M status=progress oflag=sync
    
    log "Syncing..."
    sync
fi

echo ""
log "✓ Bootable USB created successfully!"
log "You can now boot from this USB drive"
echo ""
log "Boot instructions:"
log "• Mac: Hold Option/Alt key during startup"
log "• PC: Press F12, F2, or Del during startup"
