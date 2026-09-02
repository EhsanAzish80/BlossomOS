#!/bin/sh
# Make XFCE look beautiful

echo "🌸 Installing beautiful themes and icons..."
echo ""

# Install modern themes
apk add gtk-murrine-engine
apk add adwaita-icon-theme
apk add breeze-gtk breeze-icons

# Install better fonts
apk add font-noto font-noto-emoji ttf-dejavu

# Create XFCE config for better appearance
mkdir -p /home/blossom/.config/xfce4/xfconf/xfce-perchannel-xml

# Set GTK theme to Breeze-Dark
cat > /home/blossom/.config/xfce4/xfconf/xfce-perchannel-xml/xsettings.xml << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<channel name="xsettings" version="1.0">
  <property name="Net" type="empty">
    <property name="ThemeName" type="string" value="Breeze-Dark"/>
    <property name="IconThemeName" type="string" value="breeze"/>
  </property>
  <property name="Gtk" type="empty">
    <property name="FontName" type="string" value="Noto Sans 10"/>
  </property>
</channel>
EOF

# Set window manager theme
cat > /home/blossom/.config/xfce4/xfconf/xfce-perchannel-xml/xfwm4.xml << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<channel name="xfwm4" version="1.0">
  <property name="general" type="empty">
    <property name="theme" type="string" value="Default"/>
    <property name="button_layout" type="string" value="O|HMC"/>
    <property name="button_offset" type="int" value="0"/>
    <property name="button_spacing" type="int" value="0"/>
  </property>
</channel>
EOF

# Set desktop background to dark color
cat > /home/blossom/.config/xfce4/xfconf/xfce-perchannel-xml/xfce4-desktop.xml << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<channel name="xfce4-desktop" version="1.0">
  <property name="backdrop" type="empty">
    <property name="screen0" type="empty">
      <property name="monitor0" type="empty">
        <property name="workspace0" type="empty">
          <property name="color-style" type="int" value="0"/>
          <property name="color1" type="array">
            <value type="uint" value="8738"/>
            <value type="uint" value="10794"/>
            <value type="uint" value="15677"/>
            <value type="uint" value="65535"/>
          </property>
        </property>
      </property>
    </property>
  </property>
</channel>
EOF

# Fix ownership
chown -R blossom:blossom /home/blossom/.config/xfce4

echo ""
echo "✅ Beautiful theme installed!"
echo ""
echo "Restarting XFCE in 3 seconds..."
sleep 3

# Restart display manager
pkill -9 -f xfce4-session
rc-service lightdm restart
