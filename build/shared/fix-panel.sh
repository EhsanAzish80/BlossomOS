#!/bin/sh
# Fix Panel Icon and Text

echo "🌸 Updating panel branding..."

# Kill panel
killall xfce4-panel 2>/dev/null
sleep 1

# Update panel config with just "BlossomOS" text
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
      <property name="button-title" type="string" value="BlossomOS"/>
      <property name="show-generic-names" type="bool" value="false"/>
      <property name="button-icon" type="string" value=""/>
    </property>
    <property name="plugin-2" type="string" value="tasklist"/>
    <property name="plugin-3" type="string" value="separator">
      <property name="expand" type="bool" value="true"/>
      <property name="style" type="uint" value="0"/>
    </property>
    <property name="plugin-4" type="string" value="systray"/>
    <property name="plugin-5" type="string" value="clock">
      <property name="digital-format" type="string" value="%I:%M %p"/>
    </property>
  </property>
</channel>
PANEL

chown -R blossom:blossom /home/blossom/.config

# Restart panel
su - blossom -c "xfce4-panel &" 2>/dev/null &

echo "✅ Panel updated! Look at the bottom left - it now says 'BlossomOS'"
