#!/bin/bash
# Alternative Alpine-based build (lighter weight, faster)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/build"
OUTPUT_DIR="$BUILD_DIR/out"

GREEN='\033[0;32m'
NC='\033[0m'

log() {
    echo -e "${GREEN}[BlossomOS-Alpine]${NC} $1"
}

log "Building Alpine-based BlossomOS..."
log "Note: This creates a lighter, faster booting system"

mkdir -p "$OUTPUT_DIR"

# Download Alpine extended ISO as base
ALPINE_VERSION="3.19"
ALPINE_ISO="alpine-extended-$ALPINE_VERSION-x86_64.iso"
ALPINE_URL="https://dl-cdn.alpinelinux.org/alpine/v3.19/releases/x86_64/$ALPINE_ISO"

if [ ! -f "$OUTPUT_DIR/$ALPINE_ISO" ]; then
    log "Downloading Alpine Linux..."
    curl -L "$ALPINE_URL" -o "$OUTPUT_DIR/$ALPINE_ISO"
fi

# Create customization script
cat > "$OUTPUT_DIR/customize-alpine.sh" << 'CUSTOMEOF'
#!/bin/sh
# BlossomOS Alpine Setup Script

echo "=== BlossomOS Alpine Customization ==="

# Update repositories
setup-apkrepos -1

# Install base system
apk update
apk add alpine-base linux-lts

# Install GUI
apk add xorg-server xfce4 xfce4-terminal lightdm-gtk-greeter
apk add xfce4-whiskermenu-plugin xfce4-pulseaudio-plugin
apk add papirus-icon-theme arc-theme
apk add picom

# Install development tools
apk add build-base git python3 py3-pip

# Install AI dependencies (lightweight)
apk add py3-numpy py3-requests

# Enable services
rc-update add lightdm default
rc-update add networkmanager default

echo "BlossomOS customization complete!"
CUSTOMEOF

chmod +x "$OUTPUT_DIR/customize-alpine.sh"

log "Alpine base downloaded: $OUTPUT_DIR/$ALPINE_ISO"
log "Customization script: $OUTPUT_DIR/customize-alpine.sh"
log "To install:"
log "1. Boot the Alpine ISO"
log "2. Copy customize-alpine.sh to the system"
log "3. Run: sh customize-alpine.sh"
log "4. Run: setup-alpine"
