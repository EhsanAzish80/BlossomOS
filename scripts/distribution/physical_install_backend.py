#!/usr/bin/env python3
"""Frozen Phase 11 installer backend for the first physical target only."""

from __future__ import annotations

import fcntl
import hashlib
import json
import os
import shutil
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
ROOTFS_DIGEST = Path("/root/blossom-rootfs.tar.zst.sha256")
MOUNT = Path("/mnt/blossom-install")
MOUNT_B = Path("/mnt/blossom-install-b")
STATE_DIRECTORIES = {
    "home": "home",
    "var/lib/blossom": "blossom",
    "var/log": "log",
    "etc/NetworkManager/system-connections": "networkmanager-connections",
    "var/lib/bluetooth": "bluetooth",
}
STATE_FILES = {
    "etc/passwd": "identity/passwd",
    "etc/shadow": "identity/shadow",
    "etc/group": "identity/group",
    "etc/gshadow": "identity/gshadow",
    "etc/machine-id": "machine/machine-id",
    "etc/hostname": "machine/hostname",
}
LSBLK_TARGET = (
    "lsblk",
    "--json",
    "--bytes",
    "--tree",
    "--output",
    "PATH,SIZE,MODEL,TRAN,RM,MOUNTPOINTS",
    TARGET["path"],
)


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


def _stable_disk_path(device: str) -> str:
    expected = os.path.realpath(device)
    by_id = Path("/dev/disk/by-id")
    try:
        candidates = sorted(by_id.iterdir())
    except OSError as error:
        raise BackendError("stable disk identity is unavailable") from error
    preferred = sorted(
        (entry for entry in candidates if os.path.realpath(entry) == expected),
        key=lambda entry: (not entry.name.startswith("wwn-"), entry.name),
    )
    if not preferred:
        raise BackendError("target has no stable /dev/disk/by-id identity")
    return str(preferred[0])


def _verify_rootfs() -> None:
    try:
        expected = ROOTFS_DIGEST.read_text(encoding="ascii").strip()
    except (OSError, UnicodeError) as error:
        raise BackendError("reviewed root filesystem digest is missing") from error
    if len(expected) != 64 or any(character not in "0123456789abcdef" for character in expected):
        raise BackendError("reviewed root filesystem digest is invalid")
    digest = hashlib.sha256()
    try:
        with ROOTFS.open("rb") as source:
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as error:
        raise BackendError("reviewed root filesystem archive cannot be read") from error
    if digest.hexdigest() != expected:
        raise BackendError("reviewed root filesystem digest does not match")


def _recheck_disk_identity(fd: int, disk: str, expected_rdev: int) -> None:
    retained = os.fstat(fd)
    try:
        current = os.stat(disk, follow_symlinks=True)
    except OSError as error:
        raise BackendError("stable disk identity disappeared") from error
    if (
        not stat.S_ISBLK(retained.st_mode)
        or not stat.S_ISBLK(current.st_mode)
        or retained.st_rdev != expected_rdev
        or current.st_rdev != expected_rdev
    ):
        raise BackendError("stable disk identity changed")


def command_plan(disk: str | None = None) -> list[list[str]]:
    disk = disk or TARGET["path"]
    return [
        ["sgdisk", "--zap-all", disk],
        ["sgdisk", "--new=1:2048:+1G", "--typecode=1:ef00", "--change-name=1:BLOSSOM_EFI", disk],
        ["sgdisk", "--new=2:0:+24G", "--typecode=2:8300", "--change-name=2:BLOSSOM_ROOT_A", disk],
        ["sgdisk", "--new=3:0:+24G", "--typecode=3:8300", "--change-name=3:BLOSSOM_ROOT_B", disk],
        ["sgdisk", "--new=4:0:0", "--typecode=4:8300", "--change-name=4:BLOSSOM_STATE", disk],
        ["partprobe", disk],
        ["udevadm", "settle"],
    ]


def _partitions(run: Callable[..., subprocess.CompletedProcess[Any]], disk: str) -> dict[str, str]:
    result = run(
        ["lsblk", "--raw", "--noheadings", "--paths", "--output", "PATH,PARTLABEL", disk],
        check=True, capture_output=True, text=True, timeout=5,
    )
    found: dict[str, str] = {}
    for line in result.stdout.splitlines():
        fields = line.split(None, 1)
        if len(fields) == 2 and fields[1] in {
            "BLOSSOM_EFI", "BLOSSOM_ROOT_A", "BLOSSOM_ROOT_B", "BLOSSOM_STATE"
        }:
            if fields[1] in found:
                raise BackendError("duplicate installed partition label")
            found[fields[1]] = fields[0]
    if set(found) != {"BLOSSOM_EFI", "BLOSSOM_ROOT_A", "BLOSSOM_ROOT_B", "BLOSSOM_STATE"}:
        raise BackendError("installed partition identity is incomplete")
    return found


def _inventory() -> dict[str, Any]:
    result = subprocess.run(
        LSBLK_TARGET,
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
    provision: Callable[[Path], None] | None = None,
) -> None:
    if os.geteuid() != 0:
        raise BackendError("installer backend requires root")
    if not ROOTFS.is_file():
        raise BackendError("reviewed root filesystem archive is missing")
    _verify_rootfs()
    validate_target(target, _inventory())
    disk = _stable_disk_path(TARGET["path"])
    exclusive_fd = os.open(disk, os.O_RDONLY | os.O_EXCL | os.O_CLOEXEC)
    try:
        status = os.fstat(exclusive_fd)
        if not stat.S_ISBLK(status.st_mode):
            raise BackendError("frozen target is not a block device")
        live_size = int.from_bytes(fcntl.ioctl(exclusive_fd, BLKGETSIZE64, bytes(8)), "little")
        if live_size != TARGET["size_bytes"]:
            raise BackendError("live block size does not match frozen target")
        expected_rdev = status.st_rdev
    finally:
        os.close(exclusive_fd)
    # The exclusive descriptor is intentionally released before partition
    # rescans, mkfs, and mounts. Retain a non-exclusive identity descriptor and
    # compare st_rdev through the stable by-id path before destructive phases.
    fd = os.open(disk, os.O_RDONLY | os.O_CLOEXEC)
    try:
        _recheck_disk_identity(fd, disk, expected_rdev)
        MOUNT.mkdir(parents=True, exist_ok=True)
        MOUNT_B.mkdir(parents=True, exist_ok=True)
    except Exception:
        os.close(fd)
        raise
    mounted: list[Path] = []
    try:
        for command in command_plan(disk):
            _recheck_disk_identity(fd, disk, expected_rdev)
            run(command, check=True, timeout=900)
        _recheck_disk_identity(fd, disk, expected_rdev)
        parts = _partitions(run, disk)
        _recheck_disk_identity(fd, disk, expected_rdev)
        run(["mkfs.fat", "-F", "32", "-n", "BLOSSOM_EFI", parts["BLOSSOM_EFI"]], check=True, timeout=900)
        for label in ("BLOSSOM_ROOT_A", "BLOSSOM_ROOT_B", "BLOSSOM_STATE"):
            _recheck_disk_identity(fd, disk, expected_rdev)
            run(["mkfs.ext4", "-F", "-L", label, parts[label]], check=True, timeout=900)
        for device, destination in (
            (parts["BLOSSOM_ROOT_A"], MOUNT),
            (parts["BLOSSOM_ROOT_B"], MOUNT_B),
            (parts["BLOSSOM_STATE"], MOUNT / "state"),
            (parts["BLOSSOM_EFI"], MOUNT / "efi"),
        ):
            destination.mkdir(parents=True, exist_ok=True)
            run(["mount", device, str(destination)], check=True, timeout=60)
            mounted.append(destination)
        for destination in (MOUNT, MOUNT_B):
            run(["tar", "--xattrs", "--numeric-owner", "-I", "zstd", "-xf", str(ROOTFS), "-C", str(destination)], check=True, timeout=900)
        state = MOUNT / "state"
        for root_relative, state_relative in STATE_DIRECTORIES.items():
            source = MOUNT / root_relative
            persistent = state / state_relative
            persistent.parent.mkdir(parents=True, exist_ok=True)
            if source.is_dir() and not persistent.exists():
                shutil.copytree(source, persistent, symlinks=True)
            else:
                persistent.mkdir(parents=True, exist_ok=True)
        for root_relative, state_relative in STATE_FILES.items():
            source = MOUNT / root_relative
            persistent = state / state_relative
            persistent.parent.mkdir(parents=True, exist_ok=True)
            if not source.is_file():
                source.parent.mkdir(parents=True, exist_ok=True)
                source.touch(mode=0o600)
            shutil.copy2(source, persistent)
        for root_relative, state_relative in {**STATE_DIRECTORIES, **STATE_FILES}.items():
            destination = MOUNT / root_relative
            persistent = state / state_relative
            run(["mount", "--bind", str(persistent), str(destination)], check=True, timeout=60)
            mounted.append(destination)

        uuids: dict[str, str] = {}
        for name, label in (("efi", "BLOSSOM_EFI"), ("a", "BLOSSOM_ROOT_A"), ("b", "BLOSSOM_ROOT_B"), ("state", "BLOSSOM_STATE")):
            value = run(
                ["blkid", "-s", "UUID", "-o", "value", parts[label]],
                check=True,
                capture_output=True,
                timeout=5,
                text=True,
            ).stdout.strip()
            if not value or len(value) > 64 or any(character.isspace() for character in value):
                raise BackendError(f"installed {name} UUID is invalid")
            uuids[name] = value
        run(["bootctl", f"--esp-path={MOUNT / 'efi'}", "install"], check=True, timeout=60)
        assets = MOUNT / "efi/EFI/Linux"
        assets.mkdir(parents=True, exist_ok=True)
        for slot, source in (("a", MOUNT), ("b", MOUNT_B)):
            kernel = source / "boot/vmlinuz-linux-lts"
            initramfs = source / "boot/initramfs-linux-lts.img"
            microcode = source / "boot/intel-ucode.img"
            if not all(path.is_file() for path in (kernel, initramfs, microcode)):
                raise BackendError("installed UKI input is missing")
            uki = assets / f"blossom-{slot}.efi"
            run(
                [
                    "ukify", "build", f"--linux={kernel}",
                    f"--initrd={microcode}", f"--initrd={initramfs}",
                    f"--cmdline=root=UUID={uuids[slot]} rw blossom.slot={slot.upper()} systemd.show_status=yes",
                    f"--output={uki}",
                ],
                check=True, timeout=300,
            )
        entries = MOUNT / "efi/loader/entries"
        entries.mkdir(parents=True, exist_ok=True)
        (MOUNT / "efi/loader/loader.conf").write_text(
            "default blossom-a.conf\ntimeout 3\nconsole-mode keep\neditor no\nauto-entries no\n", encoding="utf-8"
        )
        for slot in ("a", "b"):
            (entries / f"blossom-{slot}.conf").write_text(
                f"title Blossom OS ({slot.upper()})\n"
                f"efi /EFI/Linux/blossom-{slot}.efi\n",
                encoding="utf-8",
            )
        for slot, root in (("a", MOUNT), ("b", MOUNT_B)):
            (root / "efi").mkdir(parents=True, exist_ok=True)
            (root / "state").mkdir(parents=True, exist_ok=True)
            for root_relative in STATE_DIRECTORIES:
                (root / root_relative).mkdir(parents=True, exist_ok=True)
            for root_relative in STATE_FILES:
                destination = root / root_relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                if not destination.exists():
                    destination.touch(mode=0o600)
            (root / "etc/fstab").write_text(
                f"UUID={uuids[slot]} / ext4 rw,relatime 0 1\n"
                f"UUID={uuids['efi']} /efi vfat umask=0077 0 2\n"
                f"UUID={uuids['state']} /state ext4 rw,relatime 0 2\n"
                + "".join(
                    f"/state/{state_relative} /{root_relative} none bind,x-systemd.requires-mounts-for=/state 0 0\n"
                    for root_relative, state_relative in {**STATE_DIRECTORIES, **STATE_FILES}.items()
                ),
                encoding="utf-8",
            )
        if provision is not None:
            provision(MOUNT)
        run(["sync"], check=True, timeout=60)
    finally:
        for destination in reversed(mounted):
            run(["umount", str(destination)], check=False, timeout=60)
        os.close(fd)
