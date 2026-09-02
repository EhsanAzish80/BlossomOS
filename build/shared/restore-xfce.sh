#!/bin/sh
# Restore XFCE and fix the system

echo "🌸 Restoring XFCE desktop..."

# Remove broken MATE packages
apk del mate-desktop-environment mate-terminal caja pluma 2>/dev/null

# Reinstall XFCE
apk add xfce4 xfce4-terminal thunar

# Restore lightdm config
cat > /etc/lightdm/lightdm.conf << 'EOF'
[Seat:*]
autologin-user=blossom
autologin-session=xfce
EOF

# Fix .xinitrc
echo "startxfce4" > /home/blossom/.xinitrc
chown blossom:blossom /home/blossom/.xinitrc

# Make sure lightdm is enabled
rc-update add lightdm default

echo ""
echo "✅ XFCE restored!"
echo "Rebooting in 5 seconds..."
sleep 5
reboot
