#!/bin/sh
# Quick fix - Add terminal access to BlossomOS
# Run this via wget in a TTY console

echo "🌸 Setting up terminal access..."

# Install terminal if missing
apk add xfce4-terminal

# Set up keyboard shortcut
mkdir -p /home/blossom/.config/xfce4/xfconf/xfce-perchannel-xml
cat > /home/blossom/.config/xfce4/xfconf/xfce-perchannel-xml/xfce4-keyboard-shortcuts.xml << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<channel name="xfce4-keyboard-shortcuts" version="1.0">
  <property name="commands" type="empty">
    <property name="default" type="empty">
      <property name="&lt;Alt&gt;F2" type="string" value="xfce4-appfinder --collapsed"/>
      <property name="&lt;Primary&gt;&lt;Alt&gt;t" type="string" value="xfce4-terminal"/>
      <property name="&lt;Primary&gt;&lt;Alt&gt;Delete" type="string" value="xflock4"/>
    </property>
  </property>
</channel>
EOF

# Create desktop shortcut
mkdir -p /home/blossom/Desktop
cat > /home/blossom/Desktop/Terminal.desktop << 'EOF'
[Desktop Entry]
Version=1.0
Type=Application
Name=Terminal
Comment=Terminal Emulator
Exec=xfce4-terminal
Icon=utilities-terminal
Terminal=false
StartupNotify=true
EOF

chmod +x /home/blossom/Desktop/Terminal.desktop

# Add terminal to panel
mkdir -p /home/blossom/.config/xfce4/panel
cat > /home/blossom/.config/xfce4/panel/launcher-7.rc << 'EOF'
[Entry 0]
Name=Terminal
Comment=Terminal Emulator
Exec=xfce4-terminal
Icon=utilities-terminal
Terminal=false
Type=Application
EOF

# Set ownership
chown -R blossom:blossom /home/blossom/.config
chown -R blossom:blossom /home/blossom/Desktop

echo ""
echo "✅ Done! You now have:"
echo "  • Terminal icon on Desktop"
echo "  • Ctrl+Alt+T shortcut"
echo ""
echo "Restart XFCE: killall xfce4-panel"
