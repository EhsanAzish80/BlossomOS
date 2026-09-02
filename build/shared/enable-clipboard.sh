#!/bin/sh
# Enable clipboard sharing in BlossomOS VM

echo "🌸 Installing clipboard support..."

# Install SPICE guest tools
apk add spice-vdagent

# Enable the service
rc-update add spice-vdagentd
rc-service spice-vdagentd start

# For X11 clipboard support
apk add xclip xsel

echo ""
echo "✅ Clipboard tools installed!"
echo "After GNOME is installed, clipboard will work automatically"
