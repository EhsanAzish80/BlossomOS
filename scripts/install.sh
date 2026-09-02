#!/bin/bash
# BlossomOS Installation Script
# Run this after booting the live system

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log() {
    echo -e "${GREEN}[BlossomOS Installer]${NC} $1"
}

log "Welcome to BlossomOS Installation"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo "Please run with sudo"
    exit 1
fi

# Disk selection
log "Available disks:"
lsblk -d -o NAME,SIZE,TYPE | grep disk
echo ""
read -p "Enter target disk (e.g., sda): " DISK
DISK_PATH="/dev/$DISK"

if [ ! -b "$DISK_PATH" ]; then
    echo "Invalid disk: $DISK_PATH"
    exit 1
fi

log "WARNING: This will erase $DISK_PATH"
read -p "Continue? (yes/no): " confirm
if [ "$confirm" != "yes" ]; then
    exit 0
fi

# Partition disk
log "Partitioning disk..."
parted -s "$DISK_PATH" mklabel gpt
parted -s "$DISK_PATH" mkpart ESP fat32 1MiB 512MiB
parted -s "$DISK_PATH" set 1 esp on
parted -s "$DISK_PATH" mkpart primary ext4 512MiB 100%

# Format partitions
log "Formatting partitions..."
mkfs.fat -F32 "${DISK_PATH}1"
mkfs.ext4 -F "${DISK_PATH}2"

# Mount
log "Mounting partitions..."
mount "${DISK_PATH}2" /mnt
mkdir -p /mnt/boot
mount "${DISK_PATH}1" /mnt/boot

# Install base system
log "Installing base system..."
pacstrap /mnt base base-devel linux linux-firmware grub efibootmgr networkmanager

# Generate fstab
log "Generating fstab..."
genfstab -U /mnt >> /mnt/etc/fstab

# Chroot configuration
log "Configuring system..."
arch-chroot /mnt /bin/bash << 'CHROOT'
# Set timezone
ln -sf /usr/share/zoneinfo/UTC /etc/localtime
hwclock --systohc

# Locale
echo "en_US.UTF-8 UTF-8" >> /etc/locale.gen
locale-gen
echo "LANG=en_US.UTF-8" > /etc/locale.conf

# Hostname
echo "blossomos" > /etc/hostname

# Install GUI
pacman -S --noconfirm xorg xfce4 xfce4-goodies lightdm lightdm-gtk-greeter picom

# Install tools
pacman -S --noconfirm git vim python python-pip kitty firefox

# Enable services
systemctl enable lightdm
systemctl enable NetworkManager

# Install GRUB
grub-install --target=x86_64-efi --efi-directory=/boot --bootloader-id=BlossomOS
grub-mkconfig -o /boot/grub/grub.cfg

# Create user
useradd -m -G wheel -s /bin/bash blossom
echo "blossom:blossom" | chpasswd
echo "%wheel ALL=(ALL) ALL" >> /etc/sudoers

# Copy BlossomOS files
mkdir -p /opt/blossomos
cp -r /opt/blossomos-live/* /opt/blossomos/ 2>/dev/null || true

CHROOT

log "Installation complete!"
log "Default user: blossom"
log "Default password: blossom"
log "Please change the password after first login"
echo ""
read -p "Reboot now? (yes/no): " reboot_confirm

if [ "$reboot_confirm" = "yes" ]; then
    umount -R /mnt
    reboot
else
    log "Remember to unmount: umount -R /mnt"
    log "Then reboot: reboot"
fi
