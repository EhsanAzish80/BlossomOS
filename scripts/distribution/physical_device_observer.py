#!/usr/bin/env python3
"""Produce a bounded Phase 11 guard observation without write authority."""

from __future__ import annotations

import argparse
import json
import subprocess
from typing import Any


class ObservationError(ValueError):
    """Raised when the read-only block-device inventory is not closed and clear."""


MAX_OUTPUT_BYTES = 64 * 1024
MAX_DEVICES = 8
LSBLK = (
    "/usr/bin/lsblk",
    "--bytes",
    "--json",
    "--output",
    "PATH,TYPE,MODEL,SIZE,TRAN,RM,MOUNTPOINTS",
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


def parse_lsblk(payload: bytes) -> tuple[str, list[dict[str, Any]]]:
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
        if "/" in points or "/cdrom" in points:
            live.append(path)

    if not 1 <= len(devices) <= MAX_DEVICES:
        raise ObservationError("invalid whole-disk count")
    if len(live) != 1:
        raise ObservationError("live device identity is ambiguous")
    return live[0], devices


def observe() -> tuple[str, list[dict[str, Any]]]:
    """Run one fixed, read-only inventory command and parse bounded output."""
    result = subprocess.run(LSBLK, check=False, capture_output=True, timeout=5)
    if result.returncode != 0 or result.stderr:
        raise ObservationError("read-only device inventory failed")
    return parse_lsblk(result.stdout)


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
