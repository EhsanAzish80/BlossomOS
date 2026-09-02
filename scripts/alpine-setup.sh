#!/bin/sh
# BlossomOS Complete Setup Script for Alpine Linux
# Run this inside the Alpine VM after logging in as root

echo "=== BlossomOS Setup ==="

# 1. Setup repositories
echo "Setting up package repositories..."
cat > /etc/apk/repositories << EOF
https://dl-cdn.alpinelinux.org/alpine/v3.19/main
https://dl-cdn.alpinelinux.org/alpine/v3.19/community
EOF

# 2. Update package index
echo "Updating package index..."
apk update

# 3. Install base system
echo "Installing base system..."
apk add alpine-base linux-lts

# 4. Install GUI components
echo "Installing GUI (this will take a few minutes)..."
apk add \
    xorg-server \
    xf86-video-qemu \
    xf86-input-libinput \
    xfce4 \
    xfce4-terminal \
    xfce4-screensaver \
    lightdm \
    lightdm-gtk-greeter \
    dbus \
    eudev \
    mesa-dri-gallium

# 5. Install additional tools
echo "Installing tools..."
apk add \
    sudo \
    git \
    vim \
    nano \
    curl \
    wget \
    bash \
    python3 \
    py3-pip \
    firefox

# 6. Setup services
echo "Enabling services..."
rc-update add dbus
rc-update add udev
rc-update add lightdm
rc-update add networking

# 7. Create user
echo "Creating blossom user..."
adduser -D blossom
echo "blossom:blossom" | chpasswd
addgroup blossom wheel
addgroup blossom audio
addgroup blossom video
addgroup blossom input

# Enable sudo
echo "%wheel ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

# 8. Configure XFCE for the user
mkdir -p /home/blossom/.config
chown -R blossom:blossom /home/blossom

# 9. Setup AI components directory
mkdir -p /opt/blossomos/ai-core
mkdir -p /opt/blossomos/models

echo ""
echo "=== Setup Complete! ==="
echo ""
echo "Next steps:"
echo "1. Type 'reboot' to restart"
echo "2. After reboot, login with:"
echo "   Username: blossom"
echo "   Password: blossom"
echo ""
echo "Then run: sudo /opt/blossomos/install-ai.sh"
