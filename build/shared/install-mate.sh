#!/bin/sh
# Install MATE Desktop - cleaner alternative to XFCE

echo "🌸 Installing MATE Desktop..."
echo "This will take a few minutes..."
echo ""

# Remove XFCE
echo "Removing XFCE..."
apk del xfce4 xfce4-terminal

# Install MATE Desktop
echo "Installing MATE..."
apk add mate-desktop-environment mate-terminal caja pluma

# Configure lightdm for MATE
echo "Configuring auto-login..."
cat > /etc/lightdm/lightdm.conf << 'EOF'
[Seat:*]
autologin-user=blossom
autologin-session=mate
EOF

# Set MATE as default session
echo "mate-session" > /home/blossom/.xinitrc
chown blossom:blossom /home/blossom/.xinitrc

echo ""
echo "✅ MATE Desktop installed!"
echo ""
echo "🌸 Rebooting in 5 seconds..."
sleep 5
reboot
