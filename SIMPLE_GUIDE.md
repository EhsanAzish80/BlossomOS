# BlossomOS - Easy Setup Guide

## 🚀 Quick Start (Simplified)

From now on, just use these commands:

### Start BlossomOS:
```bash
./scripts/blossom
```

Your current VM is already configured! Login with:
- **User**: blossom
- **Password**: blossom (or what you set)

### Install AI Assistant:

Inside the VM, open terminal and run:
```bash
su root
apk add python3 py3-pip git
mkdir -p /opt/blossomos
# Copy AI files from host (we'll set this up)
```

## 📦 What's Configured

- ✅ XFCE Desktop Environment
- ✅ Network (DHCP)
- ✅ User: blossom
- ✅ Development tools ready

## 🤖 Next: Add AI

The hard part (OS setup) is done! Now we just need to add the AI components.

### Quick AI Setup:

1. **Install Python packages:**
```bash
sudo apk add python3-dev build-base cmake
pip3 install llama-cpp-python
```

2. **Download a small model** (pick one):
   - Phi-3 Mini (2.3GB): Fast, good general use
   - TinyLlama (637MB): Fastest, basic help

3. **Run AI assistant:**
```bash
python3 /opt/blossomos/ai-core/blossom-ai.py
```

## 💡 Tips

- To restart VM: Close window, run `./scripts/blossom` again
- To SSH into VM: `ssh -p 2222 blossom@localhost`
- To share files: We'll set up a shared folder

## 🔧 Troubleshooting

**No GUI?**
```bash
sudo rc-service lightdm start
```

**No Network?**
```bash
sudo rc-service networking restart
```

**Reset Everything?**
Delete the disk and run setup again:
```bash
rm build/out/blossomos-disk.qcow2
./scripts/start-vm.sh
```
