#!/usr/bin/env python3
"""Produce a bounded Phase 11 guard observation without write authority."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path
from typing import Any


class ObservationError(ValueError):
    """Raised when the read-only block-device inventory is not closed and clear."""


MAX_OUTPUT_BYTES = 64 * 1024
MAX_DEVICES = 8
MAX_CMDLINE_BYTES = 4 * 1024
CMDLINE = Path("/proc/cmdline")
LSBLK = (
    "/usr/bin/lsblk",
    "--bytes",
    "--json",
    "--tree",
    "--output",
    "PATH,TYPE,MODEL,SIZE,TRAN,RM,UUID,MOUNTPOINTS",
)


def _mountpoints(node: dict[str, Any]) -> list[str]:
    value = node.get("mountpoints")
    if not isinstance(value, list) or any(
        item is not None and not isinstance(item, str) for item in value
    ):
        raise ObservationError("invalid mountpoint inventory")
    points = [item for item in value if item]
    children = node.get("children", [])
    if not isinstance(children, list):
        raise ObservationError("invalid child device inventory")
    for child in children:
        if not isinstance(child, dict):
            raise ObservationError("invalid child device inventory")
        points.extend(_mountpoints(child))
    return points


def _contains_uuid(node: dict[str, Any], expected: str) -> bool:
    value = node.get("uuid")
    if value is not None and not isinstance(value, str):
        raise ObservationError("invalid filesystem UUID inventory")
    if isinstance(value, str) and value.casefold() == expected.casefold():
        return True
    children = node.get("children", [])
    if not isinstance(children, list):
        raise ObservationError("invalid child device inventory")
    matched = False
    for child in children:
        if not isinstance(child, dict):
            raise ObservationError("invalid child device inventory")
        matched = _contains_uuid(child, expected) or matched
    return matched


def parse_archiso_search_uuid(payload: bytes) -> str | None:
    """Read one bounded ArchISO search UUID from the kernel command line."""
    if len(payload) > MAX_CMDLINE_BYTES:
        raise ObservationError("kernel command line exceeds the bounded size")
    try:
        words = payload.decode("ascii").split()
    except UnicodeDecodeError as error:
        raise ObservationError("invalid kernel command line") from error
    values = [
        word.split("=", 1)[1]
        for word in words
        if word.startswith("archisosearchuuid=")
    ]
    if not values:
        return None
    if len(values) != 1 or not re.fullmatch(r"[A-Za-z0-9._-]{1,64}", values[0]):
        raise ObservationError("invalid ArchISO search UUID")
    return values[0]


def parse_lsblk(
    payload: bytes, archiso_search_uuid: str | None = None
) -> tuple[str, list[dict[str, Any]]]:
    """Normalize only whole disks and identify one live root or live-media disk."""
    if len(payload) > MAX_OUTPUT_BYTES:
        raise ObservationError("device inventory exceeds the bounded size")
    try:
        value = json.loads(payload)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ObservationError("invalid device inventory JSON") from error
    if type(value) is not dict or set(value) != {"blockdevices"}:
        raise ObservationError("device inventory schema drift")
    nodes = value["blockdevices"]
    if not isinstance(nodes, list):
        raise ObservationError("invalid device inventory")

    devices = []
    live = []
    transport_names = {
        "ata": "sata",
        "sata": "sata",
        "nvme": "nvme",
        "usb": "usb",
    }
    for node in nodes:
        if not isinstance(node, dict) or node.get("type") != "disk":
            continue
        path = node.get("path")
        model = node.get("model")
        size = node.get("size")
        transport = transport_names.get(node.get("tran"))
        removable = node.get("rm")
        points = _mountpoints(node)
        if not isinstance(path, str) or not isinstance(model, str):
            raise ObservationError("disk identity is incomplete")
        if type(size) is not int or transport is None or removable not in (True, False, 0, 1):
            raise ObservationError("disk properties are incomplete")
        if size == 0:
            # Empty card-reader slots are reported as whole disks on the frozen
            # MacBook target but contain no addressable media and cannot be a
            # live device or destructive-test candidate.
            continue
        devices.append(
            {
                "path": path,
                "model": model.strip(),
                "size_bytes": size,
                "transport": transport,
                "removable": bool(removable),
                "mounted": bool(points),
            }
        )
        if "/" in points or "/cdrom" in points or "/run/archiso/bootmnt" in points:
            live.append(path)
        if archiso_search_uuid is not None and _contains_uuid(node, archiso_search_uuid):
            live.append(path)

    if not 1 <= len(devices) <= MAX_DEVICES:
        raise ObservationError("invalid whole-disk count")
    if len(set(live)) != 1:
        raise ObservationError("live device identity is ambiguous")
    return next(iter(set(live))), devices


def observe() -> tuple[str, list[dict[str, Any]]]:
    """Run one fixed, read-only inventory command and parse bounded output."""
    result = subprocess.run(LSBLK, check=False, capture_output=True, timeout=5)
    if result.returncode != 0 or result.stderr:
        raise ObservationError("read-only device inventory failed")
    try:
        with CMDLINE.open("rb") as stream:
            cmdline = stream.read(MAX_CMDLINE_BYTES + 1)
    except OSError as error:
        raise ObservationError("kernel command line read failed") from error
    return parse_lsblk(result.stdout, parse_archiso_search_uuid(cmdline))


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Create a read-only Phase 11 guard observation"
    )
    parser.add_argument(
        "--purpose",
        required=True,
        choices=("physical_install", "disposable_test"),
    )
    parser.add_argument(
        "--host-preflight-result",
        required=True,
        choices=("eligible_for_qualification",),
    )
    parser.add_argument("--ac-power", required=True, choices=("ready",))
    parser.add_argument("--recovery-media", required=True, choices=("ready",))
    parser.add_argument("--challenge", required=True)
    args = parser.parse_args()
    live_device, devices = observe()
    print(
        json.dumps(
            {
                "schema": 1,
                "purpose": args.purpose,
                "host_preflight_result": args.host_preflight_result,
                "ac_power": True,
                "recovery_media_ready": True,
                "live_device": live_device,
                "challenge": args.challenge,
                "devices": devices,
            },
            sort_keys=True,
            separators=(",", ":"),
        )
    )


if __name__ == "__main__":
    main()
