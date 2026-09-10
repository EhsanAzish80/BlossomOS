#!/usr/bin/env python3
"""Frozen Phase 11 installer backend for the first physical target only."""

from __future__ import annotations

import fcntl
import json
import os
import stat
import subprocess
from pathlib import Path
from typing import Any, Callable


TARGET = {
    "path": "/dev/sda",
    "model": "APPLE SSD SM0128F",
    "size_bytes": 121332826112,
    "transport": "sata",
    "purpose": "physical_install",
}
BLKGETSIZE64 = 0x80081272
ROOTFS = Path("/root/blossom-rootfs.tar.zst")
MOUNT = Path("/mnt/blossom-install")


class BackendError(RuntimeError):
    """Raised when the frozen backend cannot prove its exact target."""


def validate_target(target: dict[str, Any], inventory: dict[str, Any]) -> None:
    expected = set(TARGET) | {"challenge"}
    if type(target) is not dict or set(target) != expected:
        raise BackendError("target schema drift")
    for field, value in TARGET.items():
        if target[field] != value:
            raise BackendError(f"frozen target mismatch: {field}")
    if type(target["challenge"]) is not str or len(target["challenge"]) != 32:
        raise BackendError("invalid challenge")
    if type(inventory) is not dict or set(inventory) != {"blockdevices"}:
        raise BackendError("inventory schema drift")
    devices = inventory["blockdevices"]
    if type(devices) is not list or len(devices) != 1:
        raise BackendError("exactly one inventory root is required")
    disk = devices[0]
    if not isinstance(disk, dict):
        raise BackendError("invalid inventory root")
    observed = {
        "path": disk.get("path"),
        "model": str(disk.get("model", "")).strip(),
        "size_bytes": disk.get("size"),
        "transport": disk.get("tran"),
        "removable": disk.get("rm"),
    }
    if observed != {
        "path": TARGET["path"],
        "model": TARGET["model"],
        "size_bytes": TARGET["size_bytes"],
        "transport": TARGET["transport"],
        "removable": False,
    }:
        raise BackendError("live block inventory does not match frozen target")
    children = disk.get("children", [])
    if type(children) is not list or any(type(child) is not dict for child in children):
        raise BackendError("invalid inventory children")
    nodes = [disk, *children]
    for node in nodes:
        points = node.get("mountpoints")
        if not isinstance(points, list) or any(point is not None for point in points):
            raise BackendError("target or target partition is mounted")


def command_plan() -> list[list[str]]:
    disk = TARGET["path"]
    return [
        ["sgdisk", "--zap-all", disk],
        ["sgdisk", "--new=1:2048:+512M", "--typecode=1:ef00", "--change-name=1:BLOSSOM_EFI", disk],
        ["sgdisk", "--new=2:0:0", "--typecode=2:8300", "--change-name=2:BLOSSOM_SYSTEM", disk],
        ["partprobe", disk],
        ["udevadm", "settle"],
        ["mkfs.fat", "-F", "32", "-n", "BLOSSOM_EFI", "/dev/sda1"],
        ["mkfs.ext4", "-F", "-L", "BLOSSOM_SYSTEM", "/dev/sda2"],
        ["mount", "/dev/sda2", str(MOUNT)],
        ["mount", "/dev/sda1", str(MOUNT / "boot")],
        ["tar", "--xattrs", "--numeric-owner", "-I", "zstd", "-xf", str(ROOTFS), "-C", str(MOUNT)],
        ["bootctl", f"--esp-path={MOUNT / 'boot'}", "install"],
    ]


def _inventory() -> dict[str, Any]:
    result = subprocess.run(
        ["lsblk", "--json", "--bytes", "--output", "PATH,SIZE,MODEL,TRAN,RM,MOUNTPOINTS", TARGET["path"]],
        check=True,
        capture_output=True,
        timeout=5,
    )
    if result.stderr or len(result.stdout) > 64 * 1024:
        raise BackendError("bounded inventory failed")
    return json.loads(result.stdout)


def install(
    target: dict[str, Any],
    run: Callable[..., subprocess.CompletedProcess[bytes]] = subprocess.run,
) -> None:
    if os.geteuid() != 0:
        raise BackendError("installer backend requires root")
    if not ROOTFS.is_file():
        raise BackendError("reviewed root filesystem archive is missing")
    validate_target(target, _inventory())
    fd = os.open(TARGET["path"], os.O_RDONLY | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        status = os.fstat(fd)
        if not stat.S_ISBLK(status.st_mode):
            raise BackendError("frozen target is not a block device")
        live_size = int.from_bytes(fcntl.ioctl(fd, BLKGETSIZE64, bytes(8)), "little")
        if live_size != TARGET["size_bytes"]:
            raise BackendError("live block size does not match frozen target")
    finally:
        os.close(fd)

    MOUNT.mkdir(parents=True, exist_ok=True)
    (MOUNT / "boot").mkdir(parents=True, exist_ok=True)
    mounted_root = False
    mounted_boot = False
    try:
        for command in command_plan():
            run(command, check=True, timeout=900)
            mounted_root = mounted_root or command[:2] == ["mount", "/dev/sda2"]
            mounted_boot = mounted_boot or command[:2] == ["mount", "/dev/sda1"]
        uuids = {}
        for name, device in (("efi", "/dev/sda1"), ("root", "/dev/sda2")):
            value = run(
                ["blkid", "-s", "UUID", "-o", "value", device],
                check=True,
                capture_output=True,
                timeout=5,
                text=True,
            ).stdout.strip()
            if not value or len(value) > 64 or any(character.isspace() for character in value):
                raise BackendError(f"installed {name} UUID is invalid")
            uuids[name] = value
        entries = MOUNT / "boot/loader/entries"
        entries.mkdir(parents=True, exist_ok=True)
        (MOUNT / "boot/loader/loader.conf").write_text(
            "default blossom.conf\ntimeout 3\nconsole-mode keep\n", encoding="utf-8"
        )
        (entries / "blossom.conf").write_text(
            "title Blossom OS physical qualification\n"
            "linux /vmlinuz-linux-lts\n"
            "initrd /intel-ucode.img\n"
            "initrd /initramfs-linux-lts.img\n"
            f"options root=UUID={uuids['root']} rw systemd.show_status=yes\n",
            encoding="utf-8",
        )
        (MOUNT / "etc/fstab").write_text(
            f"UUID={uuids['root']} / ext4 rw,relatime 0 1\n"
            f"UUID={uuids['efi']} /boot vfat umask=0077 0 2\n",
            encoding="utf-8",
        )
        run(["sync"], check=True, timeout=60)
    finally:
        if mounted_boot:
            run(["umount", str(MOUNT / "boot")], check=False, timeout=60)
        if mounted_root:
            run(["umount", str(MOUNT)], check=False, timeout=60)
