#!/bin/bash
# Download and setup AI models for BlossomOS

set -e

MODEL_DIR="/opt/blossomos/models"
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log() {
    echo -e "${GREEN}[Model Setup]${NC} $1"
}

info() {
    echo -e "${BLUE}[Info]${NC} $1"
}

# Create model directory
sudo mkdir -p "$MODEL_DIR"
sudo chown -R "$USER:$USER" "$MODEL_DIR"

log "BlossomOS AI Model Setup"
echo ""
echo "Available models:"
echo "1. Phi-3 Mini (3.8B) - Fast, good for general use [Recommended]"
echo "2. Llama 3.2 (3B) - Lightweight, very fast"
echo "3. Qwen 2.5 (3B) - Strong coding capabilities"
echo "4. Mistral 7B - More capable, slower"
echo ""
read -p "Select model (1-4) [1]: " choice
choice=${choice:-1}

case $choice in
    1)
        MODEL_NAME="phi-3-mini"
        MODEL_URL="https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf/resolve/main/Phi-3-mini-4k-instruct-q4.gguf"
        ;;
    2)
        MODEL_NAME="llama-3.2-3b"
        MODEL_URL="https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf"
        ;;
    3)
        MODEL_NAME="qwen-2.5-3b"
        MODEL_URL="https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf"
        ;;
    4)
        MODEL_NAME="mistral-7b"
        MODEL_URL="https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.2-GGUF/resolve/main/mistral-7b-instruct-v0.2.Q4_K_M.gguf"
        ;;
    *)
        log "Invalid choice, using Phi-3 Mini"
        MODEL_NAME="phi-3-mini"
        MODEL_URL="https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf/resolve/main/Phi-3-mini-4k-instruct-q4.gguf"
        ;;
esac

MODEL_PATH="$MODEL_DIR/$MODEL_NAME"
mkdir -p "$MODEL_PATH"

log "Downloading $MODEL_NAME..."
info "This may take a while depending on your connection"

cd "$MODEL_PATH"

# Download with wget or curl
if command -v wget &> /dev/null; then
    wget -c "$MODEL_URL" -O model.gguf
elif command -v curl &> /dev/null; then
    curl -L -C - "$MODEL_URL" -o model.gguf
else
    log "Error: wget or curl required"
    exit 1
fi

# Install llama.cpp for model inference
log "Setting up llama.cpp..."
if [ ! -d "$MODEL_DIR/llama.cpp" ]; then
    cd "$MODEL_DIR"
    git clone https://github.com/ggerganov/llama.cpp.git
    cd llama.cpp
    make
fi

# Create wrapper script
log "Creating blossom command..."
sudo tee /usr/local/bin/blossom > /dev/null << EOF
#!/bin/bash
# BlossomOS AI Assistant Wrapper

MODEL_PATH="$MODEL_PATH/model.gguf"
LLAMA_CPP="$MODEL_DIR/llama.cpp/main"

if [ ! -f "\$MODEL_PATH" ]; then
    echo "Model not found. Run: sudo /opt/blossomos/scripts/install-models.sh"
    exit 1
fi

# Use Python frontend if available, else direct llama.cpp
if [ -f "/opt/blossomos/ai-core/blossom-ai.py" ]; then
    python3 /opt/blossomos/ai-core/blossom-ai.py "\$@"
else
    # Direct inference
    \$LLAMA_CPP -m "\$MODEL_PATH" -n 512 -p "\$*" --color
fi
EOF

sudo chmod +x /usr/local/bin/blossom

# Set environment variable
echo "export BLOSSOM_MODEL_PATH=$MODEL_PATH" | sudo tee -a /etc/profile.d/blossom.sh

log "✓ Setup complete!"
echo ""
info "You can now use the 'blossom' command:"
echo "  $ blossom"
echo ""
info "Or integrate with your shell (add to ~/.bashrc or ~/.zshrc):"
echo '  eval "$(blossom --shell-integration)"'
