#!/usr/bin/env python3
"""Classify a minimized, read-only Phase 11 physical-target observation."""

from __future__ import annotations

import argparse
import json
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
    parser.add_argument("observation", help="path to the closed observation JSON")
    args = parser.parse_args()
    with open(args.observation, encoding="utf-8") as source:
        observation = json.load(source)
    print(json.dumps(classify(observation), sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()
