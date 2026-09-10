#!/usr/bin/env python3
"""Classify a minimized, read-only Phase 11 physical-target observation."""

from __future__ import annotations

import argparse
import json
import platform
from pathlib import Path
from typing import Any


class PreflightError(ValueError):
    """Raised when an observation is malformed or outside the frozen target."""


INPUT_FIELDS = {
    "architecture",
    "firmware",
    "product_name",
    "memory_mib",
    "drm_present",
    "internal_disk_present",
}
TARGET_PRODUCT = "MacBookPro11,1"


def _read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8").strip()
    except (OSError, UnicodeError):
        return ""


def observe_host(root: Path = Path("/"), architecture: str | None = None) -> dict[str, Any]:
    """Collect only the closed, non-identifying fields needed by classify()."""
    meminfo = _read_text(root / "proc/meminfo")
    memory_kib = 0
    for line in meminfo.splitlines():
        if line.startswith("MemTotal:"):
            fields = line.split()
            if len(fields) == 3 and fields[2] == "kB" and fields[1].isdigit():
                memory_kib = int(fields[1])
            break

    drm = root / "dev/dri"
    drm_present = any(drm.glob("card[0-9]*")) or any(drm.glob("renderD[0-9]*"))
    internal_disk_present = False
    block = root / "sys/block"
    try:
        candidates = tuple(block.iterdir())
    except OSError:
        candidates = ()
    for candidate in candidates:
        if candidate.name.startswith(("loop", "ram", "zram", "dm-")):
            continue
        if _read_text(candidate / "removable") == "0" and (candidate / "device").exists():
            internal_disk_present = True
            break

    return {
        "architecture": architecture if architecture is not None else platform.machine(),
        "firmware": "uefi" if (root / "sys/firmware/efi").is_dir() else "unknown",
        "product_name": _read_text(root / "sys/class/dmi/id/product_name"),
        "memory_mib": memory_kib // 1024,
        "drm_present": drm_present,
        "internal_disk_present": internal_disk_present,
    }


def classify(observation: dict[str, Any]) -> dict[str, Any]:
    """Return a closed, content-minimized qualification result.

    This function grants no installation authority and accepts no disk path.
    """
    if type(observation) is not dict or set(observation) != INPUT_FIELDS:
        raise PreflightError("physical observation schema drift")
    if type(observation["memory_mib"]) is not int:
        raise PreflightError("memory_mib must be an integer")
    for field in ("drm_present", "internal_disk_present"):
        if type(observation[field]) is not bool:
            raise PreflightError(f"{field} must be a boolean")

    checks = {
        "architecture": observation["architecture"] == "x86_64",
        "firmware": observation["firmware"] == "uefi",
        "product": observation["product_name"] == TARGET_PRODUCT,
        "memory": observation["memory_mib"] >= 4096,
        "drm": observation["drm_present"],
        "internal_disk": observation["internal_disk_present"],
    }
    return {
        "schema": 1,
        "target": TARGET_PRODUCT,
        "result": "eligible_for_qualification" if all(checks.values()) else "ineligible",
        "checks": checks,
        "authority": "read_only_preflight_only",
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Classify a Phase 11 physical-target observation without disk authority"
    )
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--input", help="path to a closed observation JSON")
    source.add_argument(
        "--observe-host",
        action="store_true",
        help="read the minimized observation from the current Linux host",
    )
    args = parser.parse_args()
    if args.observe_host:
        observation = observe_host()
    else:
        with open(args.input, encoding="utf-8") as input_file:
            observation = json.load(input_file)
    print(json.dumps(classify(observation), sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()
