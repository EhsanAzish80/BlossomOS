#!/usr/bin/env python3
"""Validate the closed Phase 9 distribution inputs without building an image."""

from __future__ import annotations

import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
DIST = ROOT / "distribution"


def fail(message: str) -> None:
    raise SystemExit(message)


manifest_path = DIST / "manifest.json"
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
fields = {"architecture", "boot", "channel", "image_schema", "packages", "product",
          "snapshot", "ssh_enabled", "telemetry"}
if set(manifest) != fields:
    fail("distribution manifest schema drift")
if (manifest["product"], manifest["architecture"], manifest["boot"],
    manifest["channel"], manifest["image_schema"]) != (
        "blossom-os", "x86_64", "uefi", "evidence", 1):
    fail("distribution identity drift")
if manifest["ssh_enabled"] is not False or manifest["telemetry"] is not False:
    fail("unsafe distribution default")
if not re.fullmatch(r"20\d\d/(0[1-9]|1[0-2])/(0[1-9]|[12]\d|3[01])", manifest["snapshot"]):
    fail("snapshot is not a fixed date")
packages = (DIST / "archiso/packages.x86_64").read_text(encoding="utf-8").splitlines()
if len(packages) != len(set(packages)) or any(not re.fullmatch(r"[a-z0-9][a-z0-9+._-]*", p) for p in packages):
    fail("unsafe or duplicate ArchISO package")
expected_external = [p for p in manifest["packages"] if not p.startswith("blossom-")]
if packages != expected_external:
    fail("ArchISO packages differ from the closed manifest")
if "linux" not in packages:
    fail("ArchISO kernel must match the official releng UEFI boot entries")
if "mkinitcpio-archiso" not in packages:
    fail("ArchISO live root hook must be present in the installation image")
if "parted" not in packages:
    fail("ArchISO installer must provide partprobe for partition discovery")
if "zsh" not in packages:
    fail("ArchISO live root account must have its configured login shell")
profile = (DIST / "archiso/profiledef.sh").read_text(encoding="utf-8")
for required in ("iso_name=\"blossom-os\"", "arch=\"x86_64\"", "uefi-x64.systemd-boot.esp"):
    if required not in profile:
        fail(f"missing fixed image property: {required}")
if "$(" in profile or "`" in profile:
    fail("image identity contains dynamic shell evaluation")
for package in ("blossom-core", "blossom-shell"):
    pkgbuild = DIST / "packages" / package / "PKGBUILD"
    text = pkgbuild.read_text(encoding="utf-8")
    if f"pkgname={package}" not in text or "arch=('x86_64')" not in text or "license=('Apache-2.0')" not in text:
        fail(f"invalid package identity: {package}")
    if re.search(r"\b(curl|wget|git clone|sudo|systemctl enable)\b", text):
        fail(f"forbidden package side effect: {package}")
    if '"$startdir/../../.."' not in text:
        fail(f"package does not use the reviewed checkout: {package}")
workflow = (ROOT / ".github/workflows/phase9-vm-install-evidence.yml").read_text(encoding="utf-8")
for required in ("makepkg --nodeps --noconfirm", "pacman --root /evidence/rootfs",
                 "rootfs/opt/blossom/.github/workflows",
                 "BLOSSOM_PACKAGES_VERIFIED", "BLOSSOM_UPDATE_ROLLBACK_VERIFIED",
                 "BLOSSOM_UPDATE_CONFIRMATION_VERIFIED", "BLOSSOM_RECOVERY_VERIFIED"):
    if required not in workflow:
        fail(f"installed image evidence is incomplete: {required}")
tree = "\n".join(p.read_text(encoding="utf-8", errors="replace") for p in DIST.rglob("*") if p.is_file())
for forbidden in ("PermitRootLogin yes", "PasswordAuthentication yes", "NOPASSWD", "sshd.service"):
    if forbidden in tree:
        fail(f"insecure distribution input: {forbidden}")
snapshot_config = (DIST / "evidence/pacman-snapshot.conf").read_text(encoding="utf-8")
if f"archive.archlinux.org/repos/{manifest['snapshot']}" not in snapshot_config:
    fail("evidence userspace does not use the fixed Arch snapshot")
installer_unit = (DIST / "evidence/blossom-evidence-install.service").read_text(encoding="utf-8")
if "Before=multi-user.target" not in installer_unit or "After=multi-user.target" in installer_unit:
    fail("installer service has an unsafe multi-user target ordering")
print("Phase 9 distribution inputs passed.")
