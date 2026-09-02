#!/usr/bin/env python3
"""
BlossomOS AI Core
Offline LLM integration for system-level assistance
"""

import os
import sys
import json
import subprocess
from pathlib import Path
from typing import List, Dict, Optional

class BlossomAI:
    """Main AI assistant class"""
    
    def __init__(self, model_path: Optional[str] = None):
        self.model_path = model_path or os.getenv('BLOSSOM_MODEL_PATH', 
                                                    '/opt/blossomos/models/phi-3-mini')
        self.history: List[Dict] = []
        self.safety_mode = True
        self.context = self._gather_system_context()
        
    def _gather_system_context(self) -> Dict:
        """Gather system information for context"""
        context = {
            'os': 'BlossomOS (Arch Linux)',
            'user': os.getenv('USER'),
            'cwd': os.getcwd(),
            'home': os.getenv('HOME'),
        }
        
        # Get system info
        try:
            context['hostname'] = subprocess.check_output(['hostname'], text=True).strip()
            context['kernel'] = subprocess.check_output(['uname', '-r'], text=True).strip()
        except:
            pass
            
        return context
    
    def chat(self, message: str, mode: str = 'suggest') -> str:
        """
        Process user message and return response
        
        Modes:
        - suggest: Only suggest commands (safe)
        - execute: Execute approved commands
        - explain: Explain without suggesting actions
        """
        
        # Add to history
        self.history.append({'role': 'user', 'content': message})
        
        # Simple rule-based responses for now
        # TODO: Integrate actual LLM when model is loaded
        response = self._process_request(message, mode)
        
        self.history.append({'role': 'assistant', 'content': response})
        return response
    
    def _process_request(self, message: str, mode: str) -> str:
        """Process the request based on mode"""
        
        message_lower = message.lower()
        
        # Command help
        if any(word in message_lower for word in ['how to', 'how do i', 'command for']):
            return self._suggest_command(message)
        
        # System information
        if any(word in message_lower for word in ['system info', 'specs', 'hardware']):
            return self._get_system_info()
        
        # Security check
        if any(word in message_lower for word in ['security', 'vulnerability', 'firewall']):
            return self._security_advice()
        
        # Code help
        if any(word in message_lower for word in ['code', 'script', 'program', 'python', 'bash']):
            return self._coding_help(message)
        
        return "I'm here to help! I can assist with:\n" \
               "• Running system commands\n" \
               "• Coding and debugging\n" \
               "• Security and server management\n" \
               "• System configuration\n\n" \
               "Ask me anything or type 'help' for more info."
    
    def _suggest_command(self, query: str) -> str:
        """Suggest appropriate command for query"""
        query_lower = query.lower()
        
        suggestions = {
            'find file': 'fd <filename> or find . -name "<filename>"',
            'search content': 'rg "<pattern>" or grep -r "<pattern>" .',
            'disk space': 'df -h or du -sh *',
            'process': 'ps aux | grep <name> or htop',
            'network': 'ip addr or nmcli device status',
            'install': 'sudo pacman -S <package>',
            'update': 'sudo pacman -Syu',
        }
        
        for key, cmd in suggestions.items():
            if key in query_lower:
                return f"💡 Suggested command:\n```bash\n{cmd}\n```\n\nWould you like me to explain this command?"
        
        return "I can help you find the right command. Could you be more specific about what you want to do?"
    
    def _get_system_info(self) -> str:
        """Get system information"""
        try:
            info = []
            info.append(f"🖥️  System: {self.context['os']}")
            info.append(f"👤 User: {self.context['user']}")
            info.append(f"🏠 Hostname: {self.context.get('hostname', 'unknown')}")
            info.append(f"🐧 Kernel: {self.context.get('kernel', 'unknown')}")
            
            # CPU info
            cpu_info = subprocess.check_output(['lscpu'], text=True)
            cpu_model = [line for line in cpu_info.split('\n') if 'Model name' in line]
            if cpu_model:
                info.append(f"⚡ CPU: {cpu_model[0].split(':')[1].strip()}")
            
            # Memory
            mem_info = subprocess.check_output(['free', '-h'], text=True).split('\n')[1]
            mem_total = mem_info.split()[1]
            info.append(f"💾 RAM: {mem_total}")
            
            return '\n'.join(info)
        except Exception as e:
            return f"Error gathering system info: {e}"
    
    def _security_advice(self) -> str:
        """Provide security recommendations"""
        return """🔒 Security Quick Checks:

1. Check for updates:
   ```bash
   sudo pacman -Syu
   ```

2. Review active connections:
   ```bash
   ss -tuln
   ```

3. Check failed login attempts:
   ```bash
   sudo journalctl -u sshd | grep Failed
   ```

4. Firewall status:
   ```bash
   sudo iptables -L
   ```

Would you like me to run any of these checks?"""
    
    def _coding_help(self, query: str) -> str:
        """Provide coding assistance"""
        return """💻 Coding Assistant Active

I can help with:
• Writing scripts (Python, Bash, etc.)
• Debugging code
• Explaining errors
• Code review and optimization

What would you like to code today?"""
    
    def load_model(self):
        """Load the LLM model"""
        # TODO: Implement actual model loading with llama.cpp or transformers
        print(f"Loading model from {self.model_path}...")
        print("Note: Using rule-based system until model is configured")


def main():
    """CLI interface for BlossomAI"""
    
    print("🌸 BlossomOS AI Assistant")
    print("Type 'exit' to quit, 'help' for commands\n")
    
    ai = BlossomAI()
    
    while True:
        try:
            user_input = input("\n💬 You: ").strip()
            
            if not user_input:
                continue
            
            if user_input.lower() in ['exit', 'quit', 'q']:
                print("👋 Goodbye!")
                break
            
            if user_input.lower() == 'help':
                print("""
Available commands:
• Chat naturally - I'll understand your intent
• 'system info' - Show system information
• 'security check' - Security recommendations
• 'clear' - Clear conversation history
• 'exit' - Quit
""")
                continue
            
            if user_input.lower() == 'clear':
                ai.history = []
                os.system('clear')
                print("🌸 BlossomOS AI Assistant\n")
                continue
            
            response = ai.chat(user_input)
            print(f"\n🤖 Blossom: {response}")
            
        except KeyboardInterrupt:
            print("\n\n👋 Goodbye!")
            break
        except Exception as e:
            print(f"\n❌ Error: {e}")


if __name__ == '__main__':
    main()
