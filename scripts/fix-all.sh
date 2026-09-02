#!/bin/sh
# Fix Panel and Setup AI - Working Version

echo "🌸 Fixing BlossomOS interface and setting up AI..."

# 1. Kill and reset panel
killall xfce4-panel 2>/dev/null

# 2. Remove old panel config and create fresh one
rm -rf /home/blossom/.config/xfce4/panel
mkdir -p /home/blossom/.config/xfce4/panel

# 3. Create working dock-style panel at BOTTOM
cat > /home/blossom/.config/xfce4/xfconf/xfce-perchannel-xml/xfce4-panel.xml << 'PANEL'
<?xml version="1.0" encoding="UTF-8"?>
<channel name="xfce4-panel" version="1.0">
  <property name="configver" type="int" value="2"/>
  <property name="panels" type="array">
    <value type="int" value="1"/>
    <property name="panel-1" type="empty">
      <property name="position" type="string" value="p=8;x=0;y=0"/>
      <property name="length" type="uint" value="100"/>
      <property name="position-locked" type="bool" value="true"/>
      <property name="size" type="uint" value="48"/>
      <property name="plugin-ids" type="array">
        <value type="int" value="1"/>
        <value type="int" value="2"/>
        <value type="int" value="3"/>
        <value type="int" value="4"/>
        <value type="int" value="5"/>
      </property>
      <property name="mode" type="uint" value="0"/>
      <property name="background-alpha" type="uint" value="90"/>
      <property name="background-style" type="uint" value="1"/>
    </property>
  </property>
  <property name="plugins" type="empty">
    <property name="plugin-1" type="string" value="applicationsmenu">
      <property name="show-button-title" type="bool" value="true"/>
      <property name="button-title" type="string" value="🌸"/>
    </property>
    <property name="plugin-2" type="string" value="tasklist"/>
    <property name="plugin-3" type="string" value="separator">
      <property name="expand" type="bool" value="true"/>
      <property name="style" type="uint" value="0"/>
    </property>
    <property name="plugin-4" type="string" value="systray">
      <property name="known-legacy-items" type="array">
        <value type="string" value="networkmanager applet"/>
      </property>
    </property>
    <property name="plugin-5" type="string" value="clock">
      <property name="digital-format" type="string" value="%I:%M %p"/>
    </property>
  </property>
</channel>
PANEL

# 4. Install AI dependencies
echo "Installing AI dependencies..."
apk add python3 py3-pip python3-dev build-base

# 5. Create AI assistant
mkdir -p /opt/blossomos/ai-core
cat > /opt/blossomos/ai-core/blossom-ai.py << 'AICODE'
#!/usr/bin/env python3
"""
BlossomOS AI Assistant
Simple offline assistant for system tasks
"""

import os
import sys
import subprocess

def print_banner():
    print("""
    🌸 BlossomOS AI Assistant
    ========================
    Your offline Linux helper
    
    Commands:
    • 'system' - Show system info
    • 'help' - List available commands  
    • 'disk' - Show disk usage
    • 'network' - Network status
    • 'exit' - Quit
    """)

def run_command(cmd):
    """Execute system command safely"""
    try:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        return result.stdout if result.stdout else result.stderr
    except Exception as e:
        return f"Error: {e}"

def handle_query(query):
    """Process user queries"""
    query = query.lower().strip()
    
    if query in ['exit', 'quit', 'q']:
        print("👋 Goodbye!")
        sys.exit(0)
    
    elif query == 'system':
        hostname = run_command('hostname')
        kernel = run_command('uname -r')
        uptime = run_command('uptime -p')
        print(f"""
🖥️  System Information:
   Hostname: {hostname.strip()}
   Kernel: {kernel.strip()}
   Uptime: {uptime.strip()}
   OS: BlossomOS (Alpine Linux)
        """)
    
    elif query == 'disk':
        disk = run_command('df -h /')
        print(f"\n💾 Disk Usage:\n{disk}")
    
    elif query == 'network':
        ip = run_command('ip addr show')
        print(f"\n🌐 Network:\n{ip}")
    
    elif query == 'help':
        print_banner()
    
    elif 'find' in query:
        search = query.replace('find', '').strip()
        if search:
            result = run_command(f'find ~ -name "*{search}*" 2>/dev/null | head -10')
            print(f"\n🔍 Search results:\n{result}")
        else:
            print("Usage: find <filename>")
    
    else:
        print(f"""
I can help with:
• System info: type 'system'
• Disk space: type 'disk'
• Network: type 'network'
• Find files: type 'find filename'

Or ask about Linux commands!
        """)

def main():
    print_banner()
    
    while True:
        try:
            query = input("\n💬 Ask me: ").strip()
            if query:
                handle_query(query)
        except KeyboardInterrupt:
            print("\n\n👋 Goodbye!")
            break
        except Exception as e:
            print(f"❌ Error: {e}")

if __name__ == '__main__':
    main()
AICODE

chmod +x /opt/blossomos/ai-core/blossom-ai.py

# 6. Create desktop launcher for AI
cat > /home/blossom/Desktop/Blossom-AI.desktop << 'AIDESKTOP'
[Desktop Entry]
Version=1.0
Type=Application
Name=Blossom AI
Comment=AI Assistant
Exec=xfce4-terminal -e "python3 /opt/blossomos/ai-core/blossom-ai.py"
Icon=utilities-terminal
Terminal=false
Categories=System;Utility;
AIDESKTOP

chmod +x /home/blossom/Desktop/Blossom-AI.desktop

# 7. Create command-line shortcut
cat > /usr/local/bin/ai << 'AICMD'
#!/bin/sh
python3 /opt/blossomos/ai-core/blossom-ai.py
AICMD
chmod +x /usr/local/bin/ai

# 8. Set ownership
chown -R blossom:blossom /home/blossom

# 9. Restart panel
su - blossom -c "xfce4-panel &" 2>/dev/null &

echo ""
echo "✅ All done!"
echo ""
echo "🌸 BlossomOS AI is ready!"
echo ""
echo "Try it:"
echo "  • Double-click 'Blossom AI' icon on desktop"
echo "  • Or open terminal and type: ai"
echo ""
echo "The panel should now be at the bottom with 🌸 logo"
echo ""
echo "Logout and login again to see all changes"
