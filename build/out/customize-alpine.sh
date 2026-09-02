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
