"""Shared non-destructive observation context for the graphical installer."""

from __future__ import annotations

from pathlib import Path
from typing import Any


STATE_ROOT = Path("/var/lib/blossom/phase11-claims")


def observation(
    challenge: str,
    live: str,
    devices: list[dict[str, Any]],
    ac_power: bool,
) -> dict[str, Any]:
    return {
        "schema": 1,
        "purpose": "physical_install",
        "host_preflight_result": "eligible_for_qualification",
        "ac_power": ac_power,
        "live_media_present": any(device["path"] == live for device in devices),
        "live_device": live,
        "challenge": challenge,
        "devices": devices,
    }
