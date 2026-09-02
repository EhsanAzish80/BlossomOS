#!/usr/bin/env python3
"""Fail closed if the Phase 3 packaging boundary drifts."""

from pathlib import Path
import configparser
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "system" / "privileged-helper" / "packaging"

BUS_NAME = "org.blossomos.Privileged1"
OBJECT_PATH = "/org/blossomos/Privileged1"
INTERFACE = BUS_NAME
METHOD = "TryRestartBluetooth1"
ACTION = "org.blossomos.privileged1.try-restart-bluetooth"
BINARY = "/usr/lib/blossom-os/blossom-privileged-helper"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def check_activation() -> None:
    parser = configparser.ConfigParser()
    parser.optionxform = str
    parser.read(PACKAGE / f"{BUS_NAME}.service")
    section = parser["D-BUS Service"]
    require(section.get("Name") == BUS_NAME, "activation bus name drift")
    require(section.get("Exec") == BINARY, "activation executable drift")
    require(section.get("User") == "root", "activation must run as root")
    require(
        section.get("SystemdService") == "blossom-privileged-helper.service",
        "activation systemd unit drift",
    )


def check_bus_policy() -> None:
    root = ET.parse(PACKAGE / f"{BUS_NAME}.conf").getroot()
    require(root.tag == "busconfig", "invalid bus policy root")
    policies = root.findall("policy")
    root_policy = next((item for item in policies if item.get("user") == "root"), None)
    default = next((item for item in policies if item.get("context") == "default"), None)
    require(root_policy is not None and default is not None, "missing bus policies")
    require(
        any(item.get("own") == BUS_NAME for item in root_policy.findall("allow")),
        "only root policy must own the helper name",
    )
    allows = default.findall("allow")
    require(len(allows) == 1, "bus policy must expose exactly one allow rule")
    allow = allows[0]
    require(allow.get("send_destination") == BUS_NAME, "bus destination drift")
    require(allow.get("send_path") == OBJECT_PATH, "bus object path drift")
    require(allow.get("send_interface") == INTERFACE, "bus interface drift")
    require(allow.get("send_member") == METHOD, "bus method drift")


def check_polkit() -> None:
    root = ET.parse(PACKAGE / "org.blossomos.privileged1.policy").getroot()
    actions = root.findall("action")
    require(len(actions) == 1 and actions[0].get("id") == ACTION, "polkit action drift")
    defaults = actions[0].find("defaults")
    require(defaults is not None, "missing polkit defaults")
    require(defaults.findtext("allow_any") == "no", "allow_any must be no")
    require(defaults.findtext("allow_inactive") == "no", "allow_inactive must be no")
    require(defaults.findtext("allow_active") == "auth_admin", "allow_active drift")
    require(not list(PACKAGE.glob("*.rules")), "package must not ship polkit rules")


def check_systemd() -> None:
    text = (PACKAGE / "blossom-privileged-helper.service").read_text()
    required = [
        "Type=dbus",
        f"BusName={BUS_NAME}",
        f"ExecStart={BINARY}",
        "User=root",
        "CapabilityBoundingSet=\n",
        "AmbientCapabilities=\n",
        "NoNewPrivileges=yes",
        "ProtectSystem=strict",
        "ReadWritePaths=/run/blossom-privileged",
        "RestrictAddressFamilies=AF_UNIX",
        "MemoryDenyWriteExecute=yes",
        "SystemCallFilter=@system-service",
        "IPAddressDeny=any",
    ]
    for value in required:
        require(value in text, f"missing systemd hardening: {value.strip()}")
    forbidden = ["ExecStart=/bin/", "sudo", "pkexec", "bash", "sh -c"]
    for value in forbidden:
        require(value not in text, f"forbidden packaging surface: {value}")


def main() -> None:
    check_activation()
    check_bus_policy()
    check_polkit()
    check_systemd()


if __name__ == "__main__":
    main()
