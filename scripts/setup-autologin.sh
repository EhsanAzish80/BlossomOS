#!/bin/sh
# BlossomOS - Configure Auto-Login and Boot Splash
# Run this inside the VM as root

echo "🌸 Configuring BlossomOS for seamless boot..."

# Install required packages
apk add lightdm-gtk-greeter plymouth plymouth-themes

# Configure auto-login for lightdm
cat > /etc/lightdm/lightdm.conf << 'EOF'
[Seat:*]
autologin-user=blossom
autologin-user-timeout=0
autologin-session=xfce
EOF

# Configure LightDM greeter
mkdir -p /etc/lightdm/lightdm-gtk-greeter.conf.d
cat > /etc/lightdm/lightdm-gtk-greeter.conf.d/01_blossom.conf << 'EOF'
[greeter]
background=#2c3e50
theme-name=Arc-Dark
icon-theme-name=Papirus-Dark
hide-user-image=true
EOF

# Hide boot messages - quiet kernel
sed -i 's/^default_kernel_opts=.*/default_kernel_opts="quiet splash loglevel=3 vga=current"/' /etc/update-extlinux.conf
update-extlinux

# Create custom Plymouth theme
mkdir -p /usr/share/plymouth/themes/blossom
cat > /usr/share/plymouth/themes/blossom/blossom.plymouth << 'PLYTHEME'
[Plymouth Theme]
Name=BlossomOS
Description=BlossomOS Boot Theme
ModuleName=script

[script]
ImageDir=/usr/share/plymouth/themes/blossom
ScriptFile=/usr/share/plymouth/themes/blossom/blossom.script
PLYTHEME

# Create simple boot animation script
cat > /usr/share/plymouth/themes/blossom/blossom.script << 'SCRIPT'
Window.SetBackgroundTopColor(0.17, 0.24, 0.31);
Window.SetBackgroundBottomColor(0.17, 0.24, 0.31);

logo.image = Image("logo.png");
logo.sprite = Sprite(logo.image);
logo.sprite.SetPosition(Window.GetWidth() / 2 - logo.image.GetWidth() / 2,
                        Window.GetHeight() / 2 - logo.image.GetHeight() / 2);

message_sprite = Sprite();
message_sprite.SetPosition(Window.GetWidth() / 2 - 100, Window.GetHeight() * 0.9);

fun message_callback(text) {
    message_sprite.SetImage(Image.Text(text, 1, 1, 1));
}
Plymouth.SetMessageFunction(message_callback);
SCRIPT

# Create a simple logo (text-based for now)
cat > /usr/share/plymouth/themes/blossom/logo.png.txt << 'LOGO'
# Placeholder - you can replace with actual logo
LOGO

# Set Plymouth theme
plymouth-set-default-theme blossom

# Configure GRUB to be silent
cat > /etc/default/grub << 'GRUB'
GRUB_TIMEOUT=0
GRUB_CMDLINE_LINUX_DEFAULT="quiet splash loglevel=3 rd.systemd.show_status=auto rd.udev.log_priority=3"
GRUB_TERMINAL_OUTPUT="gfxterm"
GRUB_DISABLE_OS_PROBER=true
GRUB_GFXMODE=auto
GRUB_GFXPAYLOAD_LINUX=keep
GRUB

# Hide TTY messages
systemctl mask getty@tty1.service

# Auto-start X on login for console users (backup)
cat >> /home/blossom/.profile << 'PROFILE'
if [ -z "$DISPLAY" ] && [ "$XDG_VTNR" = 1 ]; then
    exec startx
fi
PROFILE

chown blossom:blossom /home/blossom/.profile

# Configure .xinitrc for XFCE
cat > /home/blossom/.xinitrc << 'XINITRC'
#!/bin/sh
exec startxfce4
XINITRC

chmod +x /home/blossom/.xinitrc
chown blossom:blossom /home/blossom/.xinitrc

# Disable console blanking
cat >> /etc/rc.local << 'RCLOCAL'
setterm -blank 0 -powerdown 0
RCLOCAL

chmod +x /etc/rc.local

# Make services start properly
rc-update add lightdm default
rc-update del agetty.tty1 default 2>/dev/null || true

echo ""
echo "✅ Configuration complete!"
echo ""
echo "Reboot now for changes to take effect:"
echo "  reboot"
echo ""
echo "After reboot, you'll get:"
echo "• Silent boot (no text messages)"
echo "• Auto-login to GUI"
echo "• Clean macOS-like experience"
