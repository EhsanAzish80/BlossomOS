# BlossomOS Architecture

## System Overview

```
┌─────────────────────────────────────────────────┐
│              User Interface Layer               │
│  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │
│  │   GUI    │  │ Terminal │  │  Blossom AI  │  │
│  │  (XFCE)  │  │  (Bash)  │  │   Assistant  │  │
│  └──────────┘  └──────────┘  └──────────────┘  │
└─────────────────────────────────────────────────┘
                      │
┌─────────────────────────────────────────────────┐
│           AI Integration Layer                  │
│  ┌──────────────────────────────────────────┐  │
│  │  BlossomAI Core (Python)                 │  │
│  │  • LLM Inference (llama.cpp/transformers)│  │
│  │  • Context Management                    │  │
│  │  • Safety & Permission System            │  │
│  └──────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
                      │
┌─────────────────────────────────────────────────┐
│            System Bridge Layer                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │
│  │ Command  │  │   File   │  │   Network    │  │
│  │ Executor │  │  System  │  │   Manager    │  │
│  └──────────┘  └──────────┘  └──────────────┘  │
└─────────────────────────────────────────────────┘
                      │
┌─────────────────────────────────────────────────┐
│         Linux System (Arch Base)                │
│  • Kernel 6.x                                   │
│  • Systemd                                      │
│  • NetworkManager                               │
│  • Pacman Package Manager                       │
└─────────────────────────────────────────────────┘
```

## Components

### 1. User Interface Layer

#### XFCE Desktop
- **Why XFCE**: Lightweight, stable, VM-friendly
- **Compositor**: Picom for smooth effects
- **Terminal**: Kitty + Alacritty options
- **Theme**: Arc + Papirus for modern look

#### AI Assistant Interface
- CLI: Direct terminal interaction
- GUI: Future GTK/Qt interface
- Shell Integration: Inline suggestions

### 2. AI Integration Layer

#### BlossomAI Core
Located in `/opt/blossomos/ai-core/`

**Responsibilities**:
- Load and manage LLM models
- Process user queries
- Maintain conversation context
- Enforce safety rules

**Modes**:
1. **Suggest Mode** (default) - Only suggest commands
2. **Explain Mode** - Educational, no actions
3. **Execute Mode** - Run approved commands with confirmation

**Safety Features**:
- Command whitelist/blacklist
- Dry-run previews
- Audit logging
- User confirmation for destructive operations

### 3. System Bridge Layer

#### Command Executor
- Validates commands before execution
- Provides sandboxed environment
- Logs all operations
- Handles stdin/stdout/stderr

#### File System Interface
- Safe file operations
- Permission checks
- Backup before modifications

#### Network Manager
- Service monitoring
- Port management
- Connection diagnostics

### 4. Models

#### Supported Models
1. **Phi-3 Mini (3.8B)** - Recommended
   - Fast inference
   - Good general knowledge
   - 4-bit quantized: ~2.3GB

2. **Llama 3.2 (3B)**
   - Very fast
   - Good for quick queries
   - 4-bit quantized: ~1.9GB

3. **Qwen 2.5 (3B)**
   - Strong coding capabilities
   - Good for development tasks

4. **Mistral 7B**
   - Most capable
   - Slower but more accurate

#### Model Location
```
/opt/blossomos/models/
├── phi-3-mini/
│   └── model.gguf
├── llama-3.2-3b/
│   └── model.gguf
└── llama.cpp/
    └── main (inference binary)
```

## Data Flow

### User Query Flow
```
User Input → BlossomAI Core → Context Analysis → LLM Inference
                                      ↓
                            Intent Classification
                                      ↓
                    ┌─────────────────┼─────────────────┐
                    ↓                 ↓                 ↓
            Command Request    Info Request    Code Request
                    ↓                 ↓                 ↓
            Suggest/Execute   Retrieve Info    Generate Code
                    ↓                 ↓                 ↓
            User Confirmation  Format Output   Return Code
                    ↓                 
            Execute Command
                    ↓
            Return Result
```

### Safety Checks
```
Command Requested
      ↓
Is it in blacklist? → YES → Deny
      ↓ NO
Is it destructive? → YES → Require confirmation
      ↓ NO
Execute with logging
```

## File Structure

```
/opt/blossomos/
├── ai-core/
│   ├── blossom-ai.py          # Main AI engine
│   ├── command_executor.py    # Safe command execution
│   ├── context_manager.py     # System context gathering
│   └── safety.py              # Security rules
├── models/
│   ├── [model-name]/
│   └── llama.cpp/
├── config/
│   ├── commands.json          # Command whitelist/blacklist
│   ├── prompts/               # System prompts
│   └── safety-rules.json      # Security policies
└── logs/
    ├── commands.log           # Command execution log
    └── ai-requests.log        # AI query log

/etc/blossomos/
├── config/
│   ├── xfce4/                 # Desktop config
│   └── picom/                 # Compositor config
└── services/
    └── blossom-ai.service     # Systemd service
```

## Security Model

### Permission Levels
1. **Read-Only**: View system info, read files
2. **Suggest**: Propose commands (no execution)
3. **Execute-Safe**: Run non-destructive commands
4. **Execute-All**: Full system access (requires sudo)

### Command Categories
- **Safe**: ls, ps, top, df, etc.
- **Moderate**: cp, mv, mkdir, etc. (confirmation required)
- **Dangerous**: rm -rf, dd, mkfs, etc. (explicit approval)
- **Blocked**: Direct kernel access, bootloader modifications

### Audit Trail
All AI-initiated commands are logged:
```
[2026-01-25 10:30:45] USER: ehsan
[2026-01-25 10:30:45] QUERY: "find large files"
[2026-01-25 10:30:46] SUGGESTED: find / -type f -size +100M
[2026-01-25 10:30:50] EXECUTED: find /home -type f -size +100M
[2026-01-25 10:30:51] STATUS: success
```

## VM Optimizations

### Display
- VirtIO GPU driver
- QEMU Guest Agent
- VirtualBox Guest Additions
- VMware open-vm-tools

### Performance
- VirtIO disk and network
- Automatic CPU pinning
- Memory balloon driver
- Shared clipboard support

## Future Enhancements

1. **Web Interface**: Access AI assistant via browser
2. **Voice Control**: Speech-to-text integration
3. **Fine-tuned Models**: Custom models for sysadmin tasks
4. **Distributed Mode**: Multi-machine management
5. **Plugin System**: Extend AI capabilities
6. **GUI Dashboard**: System monitoring with AI insights
