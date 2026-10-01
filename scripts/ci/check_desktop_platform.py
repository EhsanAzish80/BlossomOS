#!/usr/bin/env python3
"""Validate the source-only desktop platform integration contract."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


packages = read("distribution/archiso/packages.x86_64").splitlines()
manifest = json.loads(read("distribution/manifest.json"))
builder = read("scripts/distribution/build_physical_candidate.sh")
package = read("distribution/packages/blossom-shell/PKGBUILD")
session = read("system/shell/runtime/blossom-start-session")
screenshot = read("system/shell/runtime/blossom-screenshot")
live_hyprland = read("distribution/archiso/airootfs/home/blossom/.config/hypr/hyprland.conf")
installed_hyprland = read("distribution/physical-rootfs/usr/share/blossom-os/physical-home/hyprland.conf")
live_portals = read("distribution/archiso/airootfs/home/blossom/.config/xdg-desktop-portal/hyprland-portals.conf")
installed_portals = read("distribution/physical-rootfs/usr/share/blossom-os/physical-home/hyprland-portals.conf")
live_mako = read("distribution/archiso/airootfs/home/blossom/.config/mako/config")
installed_mako = read("distribution/physical-rootfs/usr/share/blossom-os/physical-home/mako.conf")
record = read("docs/DESKTOP_PLATFORM_INTEGRATION.md")

required_packages = {
    "blueman",
    "bluez",
    "bluez-utils",
    "grim",
    "lxqt-policykit",
    "libnotify",
    "mako",
    "networkmanager",
    "pipewire",
    "pipewire-alsa",
    "pipewire-pulse",
    "qt6-wayland",
    "slurp",
    "ttf-ibm-plex",
    "xdg-desktop-portal",
    "xdg-desktop-portal-gtk",
    "xdg-desktop-portal-hyprland",
    "xorg-xwayland",
    "wireplumber",
}
require(required_packages <= set(packages), "desktop integration package set is incomplete")
require(manifest["packages"][:-2] == packages, "manifest and ArchISO desktop packages differ")
for name in required_packages:
    require(name in builder, f"installed rootfs omits desktop integration package: {name}")
for name in required_packages - {"blueman", "bluez", "bluez-utils", "networkmanager",
                                 "pipewire", "pipewire-alsa", "pipewire-pulse",
                                 "slurp", "wireplumber", "xorg-xwayland"}:
    require(name in package, f"Blossom shell package omits runtime dependency: {name}")
require("IBM-Plex-OFL.txt" in package,
        "blossom-shell must ship the IBM Plex OFL licence")
require("50-blossom-fonts.conf" in package,
        "blossom-shell must ship and enable its fontconfig preference")
plex_license = read("system/shell/fonts/IBM-Plex-OFL.txt")
require("SIL OPEN FONT LICENSE Version 1.1" in plex_license and
        'Reserved Font Name "Plex"' in plex_license,
        "IBM Plex OFL licence text is incomplete")

for config in (live_portals, installed_portals):
    require("default=hyprland;gtk" in config, "portal backend order is not explicit")
    require("org.freedesktop.impl.portal.FileChooser=gtk" in config,
            "portal file chooser fallback is missing")
for config in (live_mako, installed_mako):
    for value in ("anchor=top-right", "layer=overlay", "default-timeout=7000",
                  "[mode=do-not-disturb]", "invisible=1"):
        require(value in config, f"notification configuration is incomplete: {value}")

for value in (
    "dbus-update-activation-environment --systemd",
    "blossom-polkit-agent.service",
    "mako.service",
    "xdg-desktop-portal.service",
    "xdg-desktop-portal-hyprland.service",
):
    require(value in session, f"desktop session integration is incomplete: {value}")
for config in (live_hyprland, installed_hyprland):
    require("bind = , Print, exec, /usr/local/bin/blossom-screenshot" in config,
            "screenshot binding is missing")

for value in ("set -euo pipefail", "/usr/bin/grim", "/usr/bin/notify-send", "install -d -m 0700"):
    require(value in screenshot, f"bounded screenshot behavior is missing: {value}")
for value in ("$1", "eval ", "sudo", "pkexec", "curl ", "wget "):
    require(value not in screenshot, f"screenshot helper has forbidden authority: {value}")

require("source implemented; runtime evidence pending" in record,
        "platform integration status must remain evidence-bounded")
require("Automatic locking and idle suspend are **not enabled**" in record,
        "locked-account blocker is not documented")
require("hypridle" not in packages and "hyprlock" not in packages,
        "lock software must not be enabled before account provisioning")

print("desktop platform integration source gate verified")
