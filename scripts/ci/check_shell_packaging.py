#!/usr/bin/env python3
"""Fail closed if the inactive Phase 6 shell package boundary drifts."""

from pathlib import Path
import configparser
import json

ROOT = Path(__file__).resolve().parents[2]
SHELL = ROOT / "system" / "shell"
PACKAGE = SHELL / "packaging"
LOCK = SHELL / "registry" / "arch-x86_64.lock.json"
BUS_NAME = "org.blossomos.Shell1"
BINARY = "/usr/lib/blossom-os/blossom-shell-service"
UNIT = "blossom-shell-service.service"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def check_lock() -> None:
    require(LOCK.is_file() and not LOCK.is_symlink(), "missing regular shell lock")
    data = json.loads(LOCK.read_text())
    require(set(data) == {"schema_version", "observed_at", "architecture", "packages"}, "lock schema drift")
    require(data["schema_version"] == 1, "lock version drift")
    require(data["observed_at"] == "2026-09-04", "observation date drift")
    require(data["architecture"] == "x86_64", "unsupported architecture")
    expected = {"hyprland": ("0.56.2-2", "extra"), "quickshell": ("0.3.1-1", "extra"), "systemd": ("261.2-1", "core"), "dbus-broker": ("37-3", "core")}
    packages = data["packages"]
    require(len(packages) == len(expected), "package set must remain closed")
    require(len({item.get("name") for item in packages}) == len(packages), "duplicate package")
    for item in packages:
        require(set(item) == {"name", "version", "repository", "source"}, "package schema drift")
        name = item["name"]
        require(name in expected, f"unreviewed shell package: {name}")
        require((item["version"], item["repository"]) == expected[name], f"package pin drift: {name}")
        require(item["repository"] in {"core", "extra"}, f"non-stable repository: {name}")
        require(item["source"] == f"https://archlinux.org/packages/{item['repository']}/x86_64/{name}/", f"package source drift: {name}")


def check_activation() -> None:
    parser = configparser.ConfigParser()
    parser.optionxform = str
    parser.read(PACKAGE / f"{BUS_NAME}.service")
    section = parser["D-BUS Service"]
    require(section.get("Name") == BUS_NAME, "activation bus name drift")
    require(section.get("Exec") == BINARY, "activation executable drift")
    require(section.get("SystemdService") == UNIT, "activation unit drift")
    require("User" not in section, "session activation must not select an identity")


def check_unit() -> None:
    text = (PACKAGE / UNIT).read_text()
    required = ["Type=dbus", f"BusName={BUS_NAME}", f"ExecStart={BINARY}", "Restart=no", "NoNewPrivileges=yes", "CapabilityBoundingSet=\n", "AmbientCapabilities=\n", "PrivateDevices=yes", "ProtectSystem=strict", "ProtectHome=yes", "RestrictAddressFamilies=AF_UNIX", "MemoryDenyWriteExecute=yes", "IPAddressDeny=any"]
    for value in required:
        require(value in text, f"missing shell unit boundary: {value.strip()}")
    require("[Install]" not in text, "checkpoint must not be enableable")
    for value in ["User=root", "sudo", "pkexec", "/bin/sh", "sh -c", "bash", "systemctl", "RestrictNamespaces="]:
        require(value not in text, f"forbidden shell package surface: {value}")


def main() -> None:
    require({path.name for path in PACKAGE.iterdir()} == {"README.md", UNIT, f"{BUS_NAME}.service"}, "unexpected shell package surface")
    check_lock()
    check_activation()
    check_unit()


if __name__ == "__main__":
    main()
