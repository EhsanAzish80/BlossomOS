#!/bin/bash
# Automated BlossomOS Setup - One Command Installation

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/build/out"

echo "🌸 BlossomOS - Automated Setup"
echo "================================"
echo ""

# Step 1: Check/Install QEMU
if ! command -v qemu-system-x86_64 &> /dev/null; then
    echo "Installing QEMU..."
    brew install qemu
fi

# Step 2: Download Alpine if needed
cd "$BUILD_DIR"
if [ ! -f "alpine-extended-3.19-x86_64.iso" ] || [ $(stat -f%z "alpine-extended-3.19-x86_64.iso") -lt 100000000 ]; then
    echo "Downloading Alpine Linux (980MB)..."
    curl -L "https://dl-cdn.alpinelinux.org/alpine/v3.19/releases/x86_64/alpine-extended-3.19.4-x86_64.iso" -o alpine-extended-3.19-x86_64.iso
fi

# Step 3: Create automated install script
cat > "$BUILD_DIR/auto-install.sh" << 'AUTOINSTALL'
#!/bin/sh
# Auto-installer for Alpine

echo "Starting automated installation..."

# Setup repos
cat > /etc/apk/repositories << EOF
https://dl-cdn.alpinelinux.org/alpine/v3.19/main
https://dl-cdn.alpinelinux.org/alpine/v3.19/community
EOF

apk update

# Run automated setup-alpine
setup-alpine -q << ANSWERS
us
us
blossomos
eth0
dhcp
n

mypassword
mypassword
UTC
none
1
openssh
vda
sys
y
ANSWERS

# Install GUI components
apk add xorg-server xfce4-session xfce4-panel xfce4-terminal xfce4-settings \
        lightdm lightdm-gtk-greeter dbus eudev mesa-dri-gallium \
        sudo bash python3 py3-pip git vim firefox

# Setup services
rc-update add dbus
rc-update add lightdm

# Create user
adduser -D blossom << USERPASS
blossom
blossom
USERPASS
addgroup blossom wheel
addgroup blossom video
addgroup blossom audio

# Enable sudo
echo "%wheel ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

# Create AI directory
mkdir -p /opt/blossomos/ai-core
mkdir -p /opt/blossomos/models

echo "Installation complete! Rebooting..."
reboot
AUTOINSTALL

# Step 4: Create pre-configured disk
echo ""
echo "Creating virtual machine..."
if [ ! -f "blossomos-configured.qcow2" ]; then
    qemu-img create -f qcow2 blossomos-configured.qcow2 20G
fi

# Step 5: Create easy launcher
cat > "$SCRIPT_DIR/blossom" << 'LAUNCHER'
#!/bin/bash
cd "$(dirname "$0")/../build/out"

echo "🌸 Starting BlossomOS..."

qemu-system-x86_64 \
    -name "BlossomOS" \
    -m 4G \
    -smp 2 \
    -drive file=blossomos-configured.qcow2,format=qcow2 \
    -vga std \
    -display cocoa \
    -netdev user,id=net0,hostfwd=tcp::2222-:22 \
    -device e1000,netdev=net0
LAUNCHER

chmod +x "$SCRIPT_DIR/blossom"

echo ""
echo "✅ Setup Complete!"
echo ""
echo "Next steps:"
echo "1. Run the VM: ./scripts/blossom"
echo "2. Follow the on-screen Alpine setup"
echo "3. After installation, username: blossom, password: blossom"
echo ""
echo "To make it even easier, we can create a pre-installed image."
echo "Would you like that?"
