#!/usr/bin/env python3
"""Evaluate Phase 11 physical-install prerequisites without writing a device."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from typing import Any


class GuardError(ValueError):
    """Raised when guard input is malformed or cannot identify one target."""


TOP_FIELDS = {
    "schema",
    "purpose",
    "host_preflight_result",
    "ac_power",
    "recovery_media_ready",
    "live_device",
    "challenge",
    "devices",
}
DEVICE_FIELDS = {"path", "model", "size_bytes", "transport", "removable", "mounted"}
DEVICE_PATH = re.compile(r"/dev/(?:sd[a-z]|nvme[0-9]+n[0-9]+)")
CHALLENGE = re.compile(r"[0-9a-f]{32}")
MODEL = re.compile(r"[A-Za-z0-9][A-Za-z0-9 ._+-]{0,63}")
MIN_BYTES = 32 * 1024**3
MAX_BYTES = 8 * 1024**4


def _canonical(value: dict[str, Any]) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def _validate_device(device: Any) -> dict[str, Any]:
    if type(device) is not dict or set(device) != DEVICE_FIELDS:
        raise GuardError("device schema drift")
    if not isinstance(device["path"], str) or not DEVICE_PATH.fullmatch(device["path"]):
        raise GuardError("invalid device path")
    if not isinstance(device["model"], str) or not MODEL.fullmatch(device["model"]):
        raise GuardError("invalid device model")
    if type(device["size_bytes"]) is not int or not MIN_BYTES <= device["size_bytes"] <= MAX_BYTES:
        raise GuardError("invalid device size")
    if device["transport"] not in ("sata", "nvme", "usb"):
        raise GuardError("unsupported device transport")
    for field in ("removable", "mounted"):
        if type(device[field]) is not bool:
            raise GuardError(f"{field} must be a boolean")
    return device


def evaluate(observation: dict[str, Any], confirmation: str | None = None) -> dict[str, Any]:
    """Return a guard decision only; this module has no disk-write operation."""
    if type(observation) is not dict or set(observation) != TOP_FIELDS:
        raise GuardError("guard observation schema drift")
    if observation["schema"] != 1:
        raise GuardError("unsupported guard schema")
    if observation["purpose"] not in ("physical_install", "disposable_test"):
        raise GuardError("unsupported guard purpose")
    if observation["host_preflight_result"] != "eligible_for_qualification":
        raise GuardError("host preflight is not eligible")
    for field in ("ac_power", "recovery_media_ready"):
        if type(observation[field]) is not bool:
            raise GuardError(f"{field} must be a boolean")
        if not observation[field]:
            raise GuardError(f"required prerequisite is false: {field}")
    if not isinstance(observation["live_device"], str) or not DEVICE_PATH.fullmatch(
        observation["live_device"]
    ):
        raise GuardError("invalid live device")
    if not isinstance(observation["challenge"], str) or not CHALLENGE.fullmatch(
        observation["challenge"]
    ):
        raise GuardError("invalid challenge")
    if type(observation["devices"]) is not list or not 1 <= len(observation["devices"]) <= 8:
        raise GuardError("invalid device count")

    devices = [_validate_device(device) for device in observation["devices"]]
    paths = [device["path"] for device in devices]
    if len(paths) != len(set(paths)) or observation["live_device"] not in paths:
        raise GuardError("device identity is ambiguous")
    candidates = []
    for device in devices:
        if device["path"] == observation["live_device"] or device["mounted"]:
            continue
        if observation["purpose"] == "physical_install":
            if not device["removable"] and device["transport"] in ("sata", "nvme"):
                candidates.append(device)
        elif device["transport"] == "usb":
            candidates.append(device)
    if len(candidates) != 1:
        target_kind = "internal SATA or NVMe" if observation["purpose"] == "physical_install" else "USB disposable-test"
        raise GuardError(f"exactly one unmounted {target_kind} target is required")

    target = candidates[0]
    summary = {
        "challenge": observation["challenge"],
        "model": target["model"],
        "path": target["path"],
        "purpose": observation["purpose"],
        "size_bytes": target["size_bytes"],
        "transport": target["transport"],
    }
    digest = hashlib.sha256(_canonical(summary)).hexdigest()
    expected = f"ERASE {target['path']} {digest[:16]}"
    confirmed = confirmation is not None and confirmation == expected
    return {
        "schema": 1,
        "authority": "guard_decision_only",
        "result": "guard_passed_no_write_performed" if confirmed else "confirmation_required",
        "target": summary,
        "target_digest": digest,
        "expected_confirmation": expected,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Evaluate the non-writing Phase 11 install guard")
    parser.add_argument("--input", required=True, help="path to the closed guard observation JSON")
    parser.add_argument("--confirmation", help="exact typed confirmation shown by a prior evaluation")
    args = parser.parse_args()
    with open(args.input, encoding="utf-8") as source:
        observation = json.load(source)
    print(
        json.dumps(
            evaluate(observation, args.confirmation), sort_keys=True, separators=(",", ":")
        )
    )


if __name__ == "__main__":
    main()
