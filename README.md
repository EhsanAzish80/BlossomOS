# BlossomOS

An intelligent Linux distribution with integrated offline LLM for system-level assistance.

## Features

- 🤖 **Offline AI Assistant** - Local LLM for coding, security, and system management
- 🖥️ **Modern GUI** - Lightweight XFCE desktop with Picom compositor
- 🚀 **VM Optimized** - Works seamlessly on VirtualBox, QEMU, and VMware
- 🍎 **Mac Compatible** - Bootable USB support for Mac hardware
- 🔒 **Security Focused** - Built-in security tools and AI-assisted hardening
- ⚡ **Fast & Minimal** - Based on Arch Linux for performance

## Project Structure

```
BlossomOs/
├── build/              # ISO build scripts and configuration
├── rootfs/             # Root filesystem overlay
├── config/             # System configurations
├── ai-core/            # LLM integration and system bridge
├── gui/                # Desktop environment customization
├── scripts/            # Installation and utility scripts
└── docs/               # Documentation
```

## Building

```bash
sudo ./build/build-iso.sh
```

## Testing in VM

```bash
./scripts/test-vm.sh
```

## System Requirements

- **RAM**: 4GB minimum, 8GB recommended
- **Storage**: 20GB minimum
- **CPU**: 64-bit processor with 2+ cores
- **LLM**: Runs Phi-3 (3B) or Llama 3.2 (3B-8B) models
