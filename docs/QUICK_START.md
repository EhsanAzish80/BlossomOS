# BlossomOS Quick Start Guide

## Prerequisites

### On macOS (for building)
```bash
# Install required tools
brew install qemu wget

# For building ISO (if using Arch container)
brew install docker
```

### On Linux (for building)
```bash
# Arch Linux
sudo pacman -S archiso qemu-full

# Ubuntu/Debian
sudo apt install qemu-system-x86 wget xorriso
```

## Build Options

### Option 1: Quick Test (Recommended for First Try)
Use Alpine base - lighter and faster:

```bash
cd /Users/ehsanazish/Documents/Projects/BlossomOs
./build/build-alpine.sh
```

This downloads Alpine Linux and creates a customization script. Fast and minimal.

### Option 2: Full Build (More Features)
Build complete Arch-based BlossomOS:

```bash
cd /Users/ehsanazish/Documents/Projects/BlossomOs
sudo ./build/build-iso.sh
```

This takes 30-60 minutes and creates a full-featured ISO.

## Testing

### Test in VM (Recommended)
```bash
./scripts/test-vm.sh
```

This launches QEMU with proper Mac acceleration (hvf).

### Create Bootable USB
```bash
./scripts/create-bootable-usb.sh
```

Follow prompts to write ISO to USB drive.

## First Boot

1. **Boot the ISO** (in VM or from USB)
2. **Login**: Default user/pass will be displayed
3. **Connect to WiFi**: Click network icon in panel
4. **Open Terminal**: Click terminal icon or press Ctrl+Alt+T

## Setup AI Assistant

After booting BlossomOS:

```bash
# Install AI model (required only once)
sudo /opt/blossomos/scripts/install-models.sh

# Start AI assistant
blossom
```

### AI Assistant Usage

```bash
# Interactive mode
$ blossom
💬 You: how do I find large files?
🤖 Blossom: [suggests command]

# Direct query
$ blossom "show system information"

# Get help
$ blossom help
```

## Basic Commands

```bash
# Update system
sudo pacman -Syu

# Install package
sudo pacman -S package-name

# Network info
nmcli device status

# System info
neofetch
```

## Customization

### Change Wallpaper
1. Right-click desktop
2. Desktop Settings → Background
3. Choose image

### Change Theme
1. Applications Menu → Settings → Appearance
2. Select Arc-Dark theme
3. Select Papirus icon theme

### Add Applications
```bash
# Development
sudo pacman -S code python nodejs

# Media
sudo pacman -S vlc gimp

# Office
sudo pacman -S libreoffice
```

## Troubleshooting

### VM Display Issues
```bash
# Inside VM, install guest additions
sudo pacman -S virtualbox-guest-utils
sudo systemctl enable vboxservice
reboot
```

### AI Model Not Found
```bash
# Download model manually
cd /opt/blossomos/models
wget [model-url]
```

### Network Not Working
```bash
# Restart NetworkManager
sudo systemctl restart NetworkManager

# Or use manual connection
sudo dhcpcd
```

### No GUI After Boot
```bash
# Start display manager manually
sudo systemctl start lightdm

# Check logs
journalctl -u lightdm
```

## Next Steps

1. **Configure AI**: Edit `/opt/blossomos/config/ai-settings.json`
2. **Add Shell Integration**: Add to `~/.bashrc`:
   ```bash
   alias ask='blossom'
   ```
3. **Explore Documentation**: See `/opt/blossomos/docs/`
4. **Join Community**: [Add your community links]

## Performance Tips

### For VMs
- Allocate at least 4GB RAM
- Use 2+ CPU cores
- Enable 3D acceleration
- Use VirtIO drivers

### For Physical Hardware
- Install native GPU drivers: `sudo pacman -S nvidia` or `xf86-video-amdgpu`
- Enable compositor: `picom -b`
- Disable effects if slow: Settings → Window Manager Tweaks

## Key Locations

- **AI Core**: `/opt/blossomos/ai-core/`
- **Models**: `/opt/blossomos/models/`
- **Config**: `/etc/blossomos/`
- **Logs**: `/opt/blossomos/logs/`
- **User Config**: `~/.config/blossom/`

## Support

- **Logs**: `journalctl -b` for system logs
- **AI Logs**: `tail -f /opt/blossomos/logs/ai-requests.log`
- **Report Issues**: [Your GitHub Issues link]
