#!/usr/bin/env python3
"""Verify a Blossom OS release and safely write it to removable media."""

from __future__ import annotations

import argparse
import hashlib
import json
import plistlib
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Callable


class MediaError(RuntimeError):
    pass


@dataclass(frozen=True)
class Device:
    path: str
    model: str
    size: int
    external: bool
    removable: bool

    @property
    def identity(self) -> str:
        return f"{self.path} | {self.model} | {format_size(self.size)}"


def format_size(value: int) -> str:
    for unit in ("B", "KiB", "MiB", "GiB", "TiB"):
        if value < 1024 or unit == "TiB":
            return f"{value:.1f} {unit}" if unit != "B" else f"{value} B"
        value /= 1024
    raise AssertionError


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify_release(iso: Path, checksums: Path) -> str:
    if not iso.is_file() or not checksums.is_file():
        raise MediaError("the ISO and SHA256SUMS must both be regular files")
    matches: list[str] = []
    for line in checksums.read_text(encoding="utf-8").splitlines():
        match = re.fullmatch(r"([0-9a-fA-F]{64})\s+\*?(.+)", line.strip())
        if match and Path(match.group(2)).name == iso.name:
            matches.append(match.group(1).lower())
    if len(matches) != 1:
        raise MediaError("SHA256SUMS must contain exactly one entry for the ISO")
    actual = sha256(iso)
    if actual != matches[0]:
        raise MediaError("ISO checksum verification failed")
    return actual


def _run(command: list[str]) -> bytes:
    result = subprocess.run(command, capture_output=True, timeout=30)
    if result.returncode != 0:
        raise MediaError(f"device inventory failed: {command[0]}")
    return result.stdout


def inventory_macos() -> list[Device]:
    listing = plistlib.loads(_run(["diskutil", "list", "-plist", "external", "physical"]))
    devices = []
    for item in listing.get("AllDisksAndPartitions", []):
        identifier = item.get("DeviceIdentifier")
        if not isinstance(identifier, str):
            continue
        info = plistlib.loads(_run(["diskutil", "info", "-plist", identifier]))
        size = info.get("TotalSize")
        model = info.get("MediaName") or info.get("DeviceModel")
        if bool(info.get("WholeDisk")) and not bool(info.get("Internal", True)) and isinstance(size, int) and isinstance(model, str):
            devices.append(Device(
                f"/dev/{identifier}", model.strip(), size, True,
                bool(info.get("Ejectable") or info.get("RemovableMedia")),
            ))
    return devices


def inventory_linux() -> list[Device]:
    payload = _run(["lsblk", "--bytes", "--json", "--nodeps", "--output", "PATH,TYPE,MODEL,SIZE,TRAN,RM,HOTPLUG"])
    value = json.loads(payload)
    devices = []
    for item in value.get("blockdevices", []):
        external = item.get("tran") == "usb" or bool(item.get("hotplug"))
        removable = bool(item.get("rm"))
        path, model, size = item.get("path"), item.get("model"), item.get("size")
        # USB hard drives commonly report RM=0 even though they are external
        # and hot-pluggable. Transport/hotplug is the safety boundary here.
        if item.get("type") == "disk" and external and isinstance(path, str) and isinstance(model, str) and isinstance(size, int) and size > 0:
            devices.append(Device(path, model.strip(), size, external, removable))
    return devices


def inventory() -> list[Device]:
    if sys.platform == "darwin":
        return inventory_macos()
    if sys.platform.startswith("linux"):
        return inventory_linux()
    raise MediaError("writing is currently supported on macOS and Linux only")


def choose(devices: list[Device], target: str) -> Device:
    matches = [device for device in devices if device.path == target]
    if len(matches) != 1:
        raise MediaError("target is not one uniquely observed removable external disk")
    return matches[0]


def write_image(iso: Path, device: Device) -> None:
    if sys.platform == "darwin":
        raw = "/dev/r" + Path(device.path).name
        subprocess.run(["diskutil", "unmountDisk", device.path], check=True)
        subprocess.run(["sudo", "dd", f"if={iso}", f"of={raw}", "bs=4m"], check=True)
        subprocess.run(["sync"], check=True)
        subprocess.run(["sudo", "cmp", "-n", str(iso.stat().st_size), str(iso), raw], check=True)
        subprocess.run(["diskutil", "eject", device.path], check=True)
    else:
        value = json.loads(_run([
            "lsblk", "--json", "--output", "PATH,MOUNTPOINTS", device.path
        ]))

        def mounted_paths(nodes: list[dict]) -> list[str]:
            paths = []
            for node in nodes:
                points = node.get("mountpoints") or []
                if any(points):
                    paths.append(node["path"])
                paths.extend(mounted_paths(node.get("children") or []))
            return paths

        for path in reversed(mounted_paths(value.get("blockdevices") or [])):
            subprocess.run(["sudo", "umount", path], check=True)
        subprocess.run(["sudo", "dd", f"if={iso}", f"of={device.path}", "bs=4M", "conv=fsync"], check=True)
        subprocess.run(["sync"], check=True)
        subprocess.run(["sudo", "cmp", "-n", str(iso.stat().st_size), str(iso), device.path], check=True)


def run(iso: Path, checksums: Path, target: str | None, read: Callable[[str], str] = input) -> None:
    digest = verify_release(iso, checksums)
    print(f"Verified {iso.name}: {digest}")
    devices = inventory()
    if not devices:
        raise MediaError("no removable external whole disk was found")
    print("Eligible removable external disks:")
    for device in devices:
        print(f"  {device.identity}")
    if target is None:
        print("\nInventory only. Re-run with --target /dev/... to select a disk.")
        return
    selected = choose(devices, target)
    if iso.stat().st_size > selected.size:
        raise MediaError("the selected disk is smaller than the ISO")
    challenge = f"ERASE {selected.identity}"
    print("\nThis permanently erases the selected disk.")
    if read(f"Type exactly: {challenge}\n> ") != challenge:
        raise MediaError("confirmation did not match")
    if choose(inventory(), target) != selected:
        raise MediaError("device identity changed after confirmation")
    write_image(iso, selected)
    print("Bootable Blossom OS media created successfully.")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--iso", required=True, type=Path)
    parser.add_argument("--checksums", required=True, type=Path)
    parser.add_argument("--target", help="whole removable disk path; omit for inventory only")
    args = parser.parse_args()
    try:
        run(args.iso.resolve(), args.checksums.resolve(), args.target)
    except (MediaError, OSError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        raise SystemExit(f"media creator: {error}") from error


if __name__ == "__main__":
    main()
