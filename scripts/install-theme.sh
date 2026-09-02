#!/bin/sh
# BlossomOS - Beautiful Modern Theme Setup
# Run as root inside VM

echo "🌸 Installing beautiful BlossomOS theme..."

# Install theme packages
apk add arc-theme papirus-icon-theme font-dejavu font-noto \
        font-noto-emoji gtk-murrine-engine sassc git

# Download and install WhiteSur GTK theme (macOS Big Sur style)
cd /tmp
git clone https://github.com/vinceliuice/WhiteSur-gtk-theme.git --depth=1
cd WhiteSur-gtk-theme
./install.sh -l -c Dark -t blue
./tweaks.sh -f

# Install WhiteSur icon theme
cd /tmp
git clone https://github.com/vinceliuice/WhiteSur-icon-theme.git --depth=1
cd WhiteSur-icon-theme
./install.sh -t default

# Create custom BlossomOS wallpaper (gradient)
mkdir -p /usr/share/backgrounds/blossom
cat > /usr/share/backgrounds/blossom/create-wallpaper.sh << 'WALLPAPER'
#!/bin/sh
# Create a beautiful gradient wallpaper
convert -size 2560x1440 gradient:'#667eea'-'#764ba2' \
    /usr/share/backgrounds/blossom/default.jpg 2>/dev/null || \
echo "Install imagemagick for custom wallpaper: apk add imagemagick"
WALLPAPER
chmod +x /usr/share/backgrounds/blossom/create-wallpaper.sh

# Simpler fallback - create solid color
apk add imagemagick
convert -size 2560x1440 gradient:'#1a1a2e'-'#16213e' \
    /usr/share/backgrounds/blossom/default.jpg

# Configure XFCE for blossom user
mkdir -p /home/blossom/.config/xfce4/xfconf/xfce-perchannel-xml

# XFCE Desktop settings
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
</channel>
EOF

# XFWM4 (Window Manager) settings - macOS style
cat > /home/blossom/.config/xfce4/xfconf/xfce-perchannel-xml/xfwm4.xml << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<channel name="xfwm4" version="1.0">
  <property name="general" type="empty">
    <property name="theme" type="string" value="WhiteSur-Dark"/>
    <property name="title_font" type="string" value="Noto Sans 10"/>
    <property name="button_layout" type="string" value="CHM|"/>
    <property name="borderless_maximize" type="bool" value="true"/>
    <property name="show_dock_shadow" type="bool" value="true"/>
    <property name="shadow_delta_height" type="int" value="0"/>
    <property name="shadow_delta_width" type="int" value="0"/>
    <property name="shadow_delta_x" type="int" value="0"/>
    <property name="shadow_delta_y" type="int" value="-10"/>
    <property name="shadow_opacity" type="int" value="50"/>
  </property>
</channel>
EOF

# GTK settings - theme and fonts
cat > /home/blossom/.config/gtk-3.0/settings.ini << 'EOF'
[Settings]
gtk-theme-name=WhiteSur-Dark
gtk-icon-theme-name=WhiteSur-dark
gtk-font-name=Noto Sans 10
gtk-cursor-theme-name=WhiteSur-cursors
gtk-cursor-theme-size=24
gtk-toolbar-style=GTK_TOOLBAR_ICONS
gtk-toolbar-icon-size=GTK_ICON_SIZE_LARGE_TOOLBAR
gtk-button-images=0
gtk-menu-images=0
gtk-enable-event-sounds=1
gtk-enable-input-feedback-sounds=0
gtk-xft-antialias=1
gtk-xft-hinting=1
gtk-xft-hintstyle=hintfull
gtk-xft-rgba=rgb
EOF

# Panel configuration - macOS dock style
cat > /home/blossom/.config/xfce4/xfconf/xfce-perchannel-xml/xfce4-panel.xml << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<channel name="xfce4-panel" version="1.0">
  <property name="panels" type="array">
    <value type="int" value="1"/>
    <property name="panel-1" type="empty">
      <property name="position" type="string" value="p=6;x=0;y=0"/>
      <property name="length" type="uint" value="100"/>
      <property name="position-locked" type="bool" value="true"/>
      <property name="icon-size" type="uint" value="32"/>
      <property name="size" type="uint" value="48"/>
      <property name="plugin-ids" type="array">
        <value type="int" value="1"/>
        <value type="int" value="2"/>
        <value type="int" value="3"/>
        <value type="int" value="4"/>
        <value type="int" value="5"/>
        <value type="int" value="6"/>
      </property>
      <property name="background-style" type="uint" value="1"/>
      <property name="background-alpha" type="uint" value="85"/>
      <property name="enter-opacity" type="uint" value="100"/>
      <property name="leave-opacity" type="uint" value="85"/>
    </property>
  </property>
  <property name="plugins" type="empty">
    <property name="plugin-1" type="string" value="applicationsmenu">
      <property name="button-title" type="string" value="🌸"/>
      <property name="show-button-title" type="bool" value="true"/>
    </property>
    <property name="plugin-2" type="string" value="separator">
      <property name="style" type="uint" value="0"/>
    </property>
    <property name="plugin-3" type="string" value="launcher">
      <property name="items" type="array">
        <value type="string" value="xfce4-terminal.desktop"/>
        <value type="string" value="firefox.desktop"/>
      </property>
    </property>
    <property name="plugin-4" type="string" value="separator">
      <property name="expand" type="bool" value="true"/>
      <property name="style" type="uint" value="0"/>
    </property>
    <property name="plugin-5" type="string" value="systray"/>
    <property name="plugin-6" type="string" value="clock">
      <property name="digital-format" type="string" value="%I:%M %p"/>
    </property>
  </property>
</channel>
EOF

# Terminal colors - modern and clean
mkdir -p /home/blossom/.config/xfce4/terminal
cat > /home/blossom/.config/xfce4/terminal/terminalrc << 'EOF'
[Configuration]
FontName=Monospace 11
MiscAlwaysShowTabs=FALSE
MiscBell=FALSE
MiscBordersDefault=TRUE
MiscCursorBlinks=TRUE
MiscDefaultGeometry=100x30
MiscInheritGeometry=FALSE
MiscMenubarDefault=FALSE
MiscMouseAutohide=FALSE
MiscToolbarDefault=FALSE
MiscConfirmClose=TRUE
MiscCycleTabs=TRUE
MiscTabCloseButtons=TRUE
MiscTabCloseMiddleClick=TRUE
MiscTabPosition=GTK_POS_TOP
MiscHighlightUrls=TRUE
BackgroundDarkness=0.950000
ColorForeground=#f8f8f2
ColorBackground=#1e1e1e
ColorPalette=#1e1e1e;#ff5555;#50fa7b;#f1fa8c;#bd93f9;#ff79c6;#8be9fd;#bbbbbb;#555555;#ff5555;#50fa7b;#f1fa8c;#bd93f9;#ff79c6;#8be9fd;#ffffff
EOF

# Set ownership
chown -R blossom:blossom /home/blossom/.config

# Create theme switcher script
cat > /usr/local/bin/blossom-theme << 'THEME'
#!/bin/sh
# Toggle between light and dark theme
case "$1" in
    dark)
        xfconf-query -c xsettings -p /Net/ThemeName -s "WhiteSur-Dark"
        xfconf-query -c xsettings -p /Net/IconThemeName -s "WhiteSur-dark"
        ;;
    light)
        xfconf-query -c xsettings -p /Net/ThemeName -s "WhiteSur-Light"
        xfconf-query -c xsettings -p /Net/IconThemeName -s "WhiteSur"
        ;;
    *)
        echo "Usage: blossom-theme [dark|light]"
        ;;
esac
THEME
chmod +x /usr/local/bin/blossom-theme

echo ""
echo "✅ Beautiful theme installed!"
echo ""
echo "Logout and login again to see the new theme"
echo "Or restart with: reboot"
