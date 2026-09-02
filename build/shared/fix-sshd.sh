#!/bin/sh
# Fix sshd hanging on boot

echo "Disabling sshd service..."
rc-update del sshd default 2>/dev/null
rc-update del sshd boot 2>/dev/null

echo "Adding service timeout to prevent hangs..."
echo 'rc_wait_time="3"' >> /etc/rc.conf

echo "✅ sshd disabled and boot timeout set"
echo ""
echo "Rebooting in 3 seconds..."
sleep 3
reboot
