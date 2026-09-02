#!/bin/sh
# BlossomOS - Silent Boot Configuration (Actually Works!)
# Run as root inside VM

echo "🌸 Configuring silent boot..."

# 1. Configure extlinux (Alpine's bootloader) for silent boot
cat > /etc/update-extlinux.conf << 'EOF'
overwrite=yes
vesa_menu=yes
default_kernel_opts="quiet loglevel=0 console=tty2 vga=current rd.systemd.show_status=false"
modules=sd-mod,usb-storage,ext4
root=
verbose=0
hidden=1
timeout=0
default=lts
serial_port=
serial_baud=115200
xen_opts=dom0_mem=256M
password=
EOF

update-extlinux

# 2. Disable console messages
cat >> /etc/sysctl.conf << 'EOF'
kernel.printk = 0 0 0 0
EOF

# 3. Hide getty/login prompt on tty1
sed -i 's/^tty1:/#tty1:/' /etc/inittab

# 4. Configure lightdm to start immediately
mkdir -p /etc/lightdm
cat > /etc/lightdm/lightdm.conf << 'EOF'
[Seat:*]
autologin-user=blossom
autologin-user-timeout=0
user-session=xfce
greeter-show-manual-login=false
greeter-hide-users=false
EOF

# 5. Add fbcon option to hide cursor
echo "options fbcon cursor_blink=0" > /etc/modprobe.d/fbcon.conf

echo "✅ Silent boot configured!"
echo "Reboot to see changes"
