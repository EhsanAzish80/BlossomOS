#!/usr/bin/env python3
"""Provision one installed Blossom owner without putting secrets on argv or disk."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from pathlib import Path
from typing import Callable

from scripts.distribution.installation_profile import InstallationProfile, validate_password


class ProvisionError(RuntimeError):
    """Raised when the installed identity cannot be created safely."""


def provision(
    root: Path,
    profile: InstallationProfile,
    password: str,
    confirmation: str,
    run: Callable[..., subprocess.CompletedProcess[str]] = subprocess.run,
) -> None:
    if os.geteuid() != 0:
        raise ProvisionError("installed identity provisioning requires root")
    if root == Path("/") or not (root / "etc").is_dir():
        raise ProvisionError("installation root is not mounted")
    validate_password(password, confirmation, profile.username)
    zone = root / "usr/share/zoneinfo" / profile.timezone
    if not zone.is_file():
        raise ProvisionError("selected time zone is not installed")
    groups = "audio,video,blossom-ai"
    if profile.administrator:
        groups += ",wheel"
    run(
        ["useradd", "--root", str(root), "--create-home", "--groups", groups,
         "--shell", "/bin/bash", "--comment", profile.full_name, profile.username],
        check=True, text=True, timeout=30,
    )
    # chpasswd receives the secret through a pipe. It must never appear in argv,
    # the public profile, installer logs, or retained claim evidence.
    run(
        ["chpasswd", "--root", str(root)], input=f"{profile.username}:{password}\n",
        check=True, text=True, timeout=30,
    )
    source = root / "usr/share/blossom-os/default-home"
    destination = root / "home" / profile.username
    if source.is_dir():
        shutil.copytree(source, destination, dirs_exist_ok=True)
    hyprland = destination / ".config/hypr/hyprland.conf"
    if hyprland.is_file():
        contents = hyprland.read_text(encoding="utf-8")
        contents = contents.replace("kb_layout = us", f"kb_layout = {profile.keymap}", 1)
        hyprland.write_text(contents, encoding="utf-8")
    localtime = root / "etc/localtime"
    localtime.unlink(missing_ok=True)
    localtime.symlink_to(Path("/usr/share/zoneinfo") / profile.timezone)
    (root / "etc/locale.conf").write_text(f"LANG={profile.locale}\n", encoding="utf-8")
    (root / "etc/locale.gen").write_text(f"{profile.locale} UTF-8\n", encoding="utf-8")
    (root / "etc/vconsole.conf").write_text(f"KEYMAP={profile.keymap}\n", encoding="utf-8")
    run(["arch-chroot", str(root), "locale-gen"], check=True, text=True, timeout=120)
    state = root / "var/lib/blossom/installation"
    state.mkdir(parents=True, mode=0o700, exist_ok=True)
    record = state / "owner.json"
    record.write_text(json.dumps(profile.public_record(), sort_keys=True) + "\n", encoding="utf-8")
    record.chmod(0o600)
    run(["chown", "-R", f"{profile.username}:{profile.username}", str(destination)],
        check=True, text=True, timeout=30)
