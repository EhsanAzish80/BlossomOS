#!/bin/bash
# BlossomOS ISO Builder
# Creates a bootable ISO with GUI and AI integration

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/build"
WORK_DIR="$BUILD_DIR/work"
ISO_DIR="$BUILD_DIR/iso"
OUTPUT_DIR="$BUILD_DIR/out"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log() {
    echo -e "${GREEN}[BlossomOS]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    error "Please run as root (use sudo)"
fi

log "Starting BlossomOS build process..."

# Clean previous builds
log "Cleaning previous builds..."
rm -rf "$WORK_DIR" "$ISO_DIR" "$OUTPUT_DIR"
mkdir -p "$WORK_DIR" "$ISO_DIR" "$OUTPUT_DIR"

# Install archiso if not available (for Arch-based hosts)
if command -v pacman &> /dev/null; then
    log "Checking for archiso..."
    if ! pacman -Q archiso &> /dev/null; then
        log "Installing archiso..."
        pacman -S --noconfirm archiso
    fi
fi

# Copy archiso profile
log "Setting up build profile..."
PROFILE_DIR="$BUILD_DIR/archiso"
if [ ! -d "$PROFILE_DIR" ]; then
    cp -r /usr/share/archiso/configs/releng "$PROFILE_DIR"
fi

# Customize packages
log "Configuring packages..."
cat > "$PROFILE_DIR/packages.x86_64" << 'EOF'
# Base system
base
base-devel
linux
linux-firmware
grub
efibootmgr
networkmanager
dhcpcd

# GUI Environment
xorg-server
xorg-xinit
xfce4
xfce4-goodies
lightdm
lightdm-gtk-greeter
picom
papirus-icon-theme
arc-gtk-theme

# Terminal & Shell
kitty
alacritty
zsh
fish
starship

# Development tools
git
vim
neovim
python
python-pip
gcc
make
cmake

# AI/ML dependencies
python-pytorch
python-transformers
python-numpy
python-requests

# System utilities
htop
btop
neofetch
ranger
fzf
ripgrep
fd
bat
exa

# Networking
curl
wget
openssh
nmap

# Virtualization support
open-vm-tools
virtualbox-guest-utils
qemu-guest-agent

# Multimedia
pulseaudio
pavucontrol
firefox
EOF

# Copy custom configurations
log "Copying custom configurations..."
mkdir -p "$PROFILE_DIR/airootfs/etc/skel"
mkdir -p "$PROFILE_DIR/airootfs/opt/blossomos"

# Copy AI core
cp -r "$PROJECT_ROOT/ai-core" "$PROFILE_DIR/airootfs/opt/blossomos/" || true

# Copy GUI configurations
cp -r "$PROJECT_ROOT/config" "$PROFILE_DIR/airootfs/etc/blossomos/" || true

# Build ISO
log "Building ISO (this may take a while)..."
mkarchiso -v -w "$WORK_DIR" -o "$OUTPUT_DIR" "$PROFILE_DIR"

log "Build complete!"
ISO_FILE=$(ls "$OUTPUT_DIR"/*.iso | head -n 1)
log "ISO created: $ISO_FILE"
log "Size: $(du -h "$ISO_FILE" | cut -f1)"

# Create checksums
log "Creating checksums..."
cd "$OUTPUT_DIR"
sha256sum *.iso > SHA256SUMS
md5sum *.iso > MD5SUMS

log "✓ BlossomOS ISO ready for testing!"
log "Test with: ./scripts/test-vm.sh"
