#!/bin/sh
# Quick AI components installer
# Run after GUI is working: sudo sh /opt/blossomos/install-ai.sh

echo "=== Installing BlossomOS AI Components ==="

# Install Python dependencies
apk add --no-cache \
    python3-dev \
    py3-numpy \
    py3-requests \
    build-base \
    cmake \
    git

# Install pip packages
pip3 install --break-system-packages \
    llama-cpp-python \
    transformers \
    torch --index-url https://download.pytorch.org/whl/cpu

# Download a small model (Phi-3 Mini)
echo ""
echo "Download AI model? This will download ~2GB"
read -p "Download now? (y/n): " download

if [ "$download" = "y" ]; then
    echo "Downloading Phi-3 Mini model..."
    mkdir -p /opt/blossomos/models/phi-3-mini
    cd /opt/blossomos/models/phi-3-mini
    
    wget -c "https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf/resolve/main/Phi-3-mini-4k-instruct-q4.gguf" \
        -O model.gguf
    
    echo "Model downloaded!"
else
    echo "Skipped model download. Run manually later:"
    echo "  cd /opt/blossomos/models && wget [model-url]"
fi

# Create blossom command
cat > /usr/local/bin/blossom << 'EOF'
#!/bin/sh
python3 /opt/blossomos/ai-core/blossom-ai.py "$@"
EOF

chmod +x /usr/local/bin/blossom

echo ""
echo "=== AI Setup Complete! ==="
echo "Type 'blossom' to start the AI assistant"
