#!/bin/sh
# BlossomOS - Complete Polish Script
# Fix boot messages, add dock, fix icons, prepare for AI

echo "🌸 Polishing BlossomOS..."

# 1. REALLY hide boot messages - modify OpenRC itself
cat >> /etc/rc.conf << 'EOF'
rc_verbose=no
rc_parallel=YES
rc_logger=YES
EOF

# Hide all OpenRC messages completely
sed -i 's/^#rc_verbose=.*/rc_verbose=no/' /etc/rc.conf || true

# Make kernel completely silent
cat > /etc/update-extlinux.conf << 'EOF'
overwrite=yes
vesa_menu=yes
default_kernel_opts="quiet loglevel=0 console=ttyS0 vt.global_cursor_default=0"
modules=sd-mod,usb-storage,ext4
root=
verbose=0
hidden=1
timeout=0
default=lts
password=
EOF
update-extlinux

# Hide all systemd/openrc output
cat >> /etc/sysctl.conf << 'EOF'
kernel.printk = 0 0 0 0
kernel.printk_devkmsg = off
EOF

# 2. Create a proper dock-style panel at bottom
mkdir -p /home/blossom/.config/xfce4/xfconf/xfce-perchannel-xml
cat > /home/blossom/.config/xfce4/xfconf/xfce-perchannel-xml/xfce4-panel.xml << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<channel name="xfce4-panel" version="1.0">
  <property name="configver" type="int" value="2"/>
  <property name="panels" type="array">
    <value type="int" value="1"/>
    <property name="panel-1" type="empty">
      <property name="position" type="string" value="p=6;x=512;y=768"/>
      <property name="length" type="uint" value="70"/>
      <property name="position-locked" type="bool" value="true"/>
      <property name="size" type="uint" value="56"/>
      <property name="plugin-ids" type="array">
        <value type="int" value="1"/>
        <value type="int" value="2"/>
        <value type="int" value="3"/>
        <value type="int" value="4"/>
        <value type="int" value="5"/>
        <value type="int" value="6"/>
      </property>
      <property name="background-style" type="uint" value="1"/>
      <property name="background-alpha" type="uint" value="80"/>
      <property name="mode" type="uint" value="0"/>
      <property name="autohide-behavior" type="uint" value="0"/>
    </property>
  </property>
  <property name="plugins" type="empty">
    <property name="plugin-1" type="string" value="applicationsmenu">
      <property name="button-icon" type="string" value="distributor-logo"/>
      <property name="show-button-title" type="bool" value="false"/>
      <property name="button-title" type="string" value=""/>
    </property>
    <property name="plugin-2" type="string" value="separator">
      <property name="style" type="uint" value="0"/>
      <property name="expand" type="bool" value="false"/>
    </property>
    <property name="plugin-3" type="string" value="launcher">
      <property name="items" type="array">
        <value type="string" value="xfce4-terminal.desktop"/>
        <value type="string" value="thunar.desktop"/>
      </property>
    </property>
    <property name="plugin-4" type="string" value="separator">
      <property name="style" type="uint" value="0"/>
      <property name="expand" type="bool" value="true"/>
    </property>
    <property name="plugin-5" type="string" value="systray">
      <property name="square-icons" type="bool" value="true"/>
      <property name="icon-size" type="uint" value="0"/>
    </property>
    <property name="plugin-6" type="string" value="clock">
      <property name="digital-format" type="string" value="%I:%M %p"/>
      <property name="mode" type="uint" value="2"/>
    </property>
  </property>
</channel>
EOF

# 3. Create a beautiful custom icon for the menu
mkdir -p /usr/share/pixmaps
cat > /usr/share/pixmaps/blossom-logo.svg << 'SVG'
<?xml version="1.0" encoding="UTF-8"?>
<svg width="64" height="64" viewBox="0 0 64 64" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="grad1" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:#667eea;stop-opacity:1" />
      <stop offset="100%" style="stop-color:#764ba2;stop-opacity:1" />
    </linearGradient>
  </defs>
  <circle cx="32" cy="32" r="30" fill="url(#grad1)"/>
  <text x="32" y="42" font-family="Arial" font-size="36" fill="white" text-anchor="middle">🌸</text>
</svg>
SVG

# Create PNG version for better compatibility
apk add imagemagick 2>/dev/null || true
convert /usr/share/pixmaps/blossom-logo.svg /usr/share/pixmaps/blossom-logo.png 2>/dev/null || true

# 4. Update desktop file for better branding
cat > /usr/share/applications/blossom-ai.desktop << 'EOF'
[Desktop Entry]
Version=1.0
Type=Application
Name=Blossom AI
Comment=AI Assistant
Exec=python3 /opt/blossomos/ai-core/blossom-ai.py
Icon=blossom-logo
Terminal=true
Categories=System;Utility;
EOF

# 5. Install file manager and apps if missing
apk add thunar firefox

# 6. Configure nice wallpaper
mkdir -p /usr/share/backgrounds/blossom
apk add imagemagick
convert -size 2560x1440 \
  -define gradient:angle=135 \
  gradient:'#1a1a2e'-'#16213e' \
  /usr/share/backgrounds/blossom/default.jpg

# Set wallpaper
cat > /home/blossom/.config/xfce4/xfconf/xfce-perchannel-xml/xfce4-desktop.xml << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<channel name="xfce4-desktop" version="1.0">
  <property name="backdrop" type="empty">
    <property name="screen0" type="empty">
      <property name="monitorVirtual-1" type="empty">
        <property name="workspace0" type="empty">
          <property name="color-style" type="int" value="0"/>
          <property name="image-style" type="int" value="5"/>
          <property name="last-image" type="string" value="/usr/share/backgrounds/blossom/default.jpg"/>
        </property>
      </property>
    </property>
  </property>
  <property name="desktop-icons" type="empty">
    <property name="file-icons" type="empty">
      <property name="show-home" type="bool" value="true"/>
      <property name="show-trash" type="bool" value="false"/>
      <property name="show-filesystem" type="bool" value="true"/>
      <property name="show-removable" type="bool" value="false"/>
    </property>
    <property name="icon-size" type="uint" value="48"/>
  </property>
</channel>
EOF

# 7. Prepare AI directory
mkdir -p /opt/blossomos/ai-core
mkdir -p /opt/blossomos/models

# Set ownership
chown -R blossom:blossom /home/blossom/.config

echo ""
echo "✅ All fixes applied!"
echo ""
echo "Now downloading AI components..."

# Download AI assistant
wget -q http://10.0.2.2:8080/blossom-ai.py -O /opt/blossomos/ai-core/blossom-ai.py 2>/dev/null || \
  echo "Note: Start web server on host to download AI code"

echo ""
echo "Reboot to see all changes: reboot"
echo "Or restart panel: killall xfce4-panel && xfce4-panel &"
