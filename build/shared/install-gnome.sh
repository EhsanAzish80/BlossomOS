#!/bin/sh
# Switch BlossomOS to GNOME Desktop

echo "🌸 Installing GNOME - This will take a few minutes..."

# 1. Install GNOME and dependencies
apk add gnome-core gnome-terminal gnome-tweaks \
        gdm dbus elogind polkit-elogind \
        networkmanager-wifi gnome-shell \
        gnome-control-center gnome-backgrounds

# 2. Install additional GNOME apps
apk add nautilus gnome-calculator gnome-system-monitor \
        gnome-screenshot gedit

# 3. Disable old desktop manager
rc-update del lightdm default 2>/dev/null

# 4. Enable GNOME services
rc-update add dbus
rc-update add gdm
rc-update add elogind
rc-update add networkmanager

# 5. Configure auto-login for GNOME
mkdir -p /etc/gdm
cat > /etc/gdm/custom.conf << 'GDM'
[daemon]
AutomaticLoginEnable=true
AutomaticLogin=blossom

[security]

[xdmcp]

[chooser]

[debug]
GDM

# 6. Set GNOME as default session
mkdir -p /home/blossom/.config
echo "gnome" > /home/blossom/.config/default-session

# 7. Configure GNOME settings for blossom user
mkdir -p /home/blossom/.config/dconf
cat > /home/blossom/.config/dconf/user.d/01-blossom << 'DCONF'
[org/gnome/desktop/interface]
gtk-theme='Adwaita-dark'
icon-theme='Adwaita'
clock-format='12h'

[org/gnome/desktop/background]
picture-uri='file:///usr/share/backgrounds/gnome/adwaita-day.jpg'
primary-color='#000000'

[org/gnome/shell]
favorite-apps=['org.gnome.Terminal.desktop', 'org.gnome.Nautilus.desktop', 'firefox.desktop']

[org/gnome/desktop/wm/preferences]
button-layout='close,minimize,maximize:'
DCONF

# 8. Set ownership
chown -R blossom:blossom /home/blossom

echo ""
echo "✅ GNOME installed successfully!"
echo ""
echo "🔄 Rebooting in 5 seconds..."
echo "After reboot, you'll see the beautiful GNOME desktop!"
sleep 5
reboot
