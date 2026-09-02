#!/bin/sh
# Type this single line in the VM:
echo "https://dl-cdn.alpinelinux.org/alpine/v3.19/main" > /etc/apk/repositories && echo "https://dl-cdn.alpinelinux.org/alpine/v3.19/community" >> /etc/apk/repositories && apk update && apk add xfce4 lightdm dbus eudev xfce4-terminal && rc-update add dbus && rc-update add lightdm && adduser -D blossom && echo "blossom:blossom" | chpasswd && reboot
