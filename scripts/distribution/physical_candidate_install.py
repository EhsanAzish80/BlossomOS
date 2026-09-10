#!/usr/bin/env python3
"""Interactive, target-bound entrypoint for the frozen physical candidate."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, Callable

from scripts.distribution.physical_device_observer import observe
from scripts.distribution.physical_install_guard import evaluate
from scripts.distribution.physical_preflight import classify, observe_host
from scripts.distribution.run_physical_install import execute


STATE_ROOT = Path("/var/lib/blossom/phase11-claims")


class CandidateError(RuntimeError):
    """Raised when the physical candidate cannot preserve its closed boundary."""


def _observation(challenge: str, live: str, devices: list[dict[str, Any]]) -> dict[str, Any]:
    return {
        "schema": 1,
        "purpose": "physical_install",
        "host_preflight_result": "eligible_for_qualification",
        "ac_power": True,
        "recovery_media_ready": True,
        "live_device": live,
        "challenge": challenge,
        "devices": devices,
    }


def run_interactive(
    read: Callable[[str], str] = input,
    host_observer: Callable[[], dict[str, Any]] = observe_host,
    device_observer: Callable[[], tuple[str, list[dict[str, Any]]]] = observe,
    executor: Callable[..., dict[str, Any]] = execute,
    challenge_factory: Callable[[int], bytes] = os.urandom,
) -> dict[str, Any]:
    if os.geteuid() != 0:
        raise CandidateError("run this command with sudo")
    preflight = classify(host_observer())
    if preflight["result"] != "eligible_for_qualification":
        raise CandidateError("this is not the frozen qualified host")
    if read("Type AC-READY after connecting power: ") != "AC-READY":
        raise CandidateError("AC-power assertion cancelled")
    if read("Type RECOVERY-READY while this boot media is connected: ") != "RECOVERY-READY":
        raise CandidateError("recovery-media assertion cancelled")

    challenge = challenge_factory(16).hex()
    live, devices = device_observer()
    initial = _observation(challenge, live, devices)
    decision = evaluate(initial)
    target = decision["target"]
    print(f"Target: {target['path']} | {target['model']} | {target['size_bytes']} bytes")
    print("This permanently erases the target and consumes this attempt.")
    confirmation = read(f"Type exactly: {decision['expected_confirmation']}\n> ")

    current_live, current_devices = device_observer()
    current = _observation(challenge, current_live, current_devices)
    STATE_ROOT.mkdir(parents=True, exist_ok=True)
    initial_path = STATE_ROOT / f"{decision['target_digest']}.initial.json"
    current_path = STATE_ROOT / f"{decision['target_digest']}.current.json"
    initial_path.write_text(json.dumps(initial, sort_keys=True, separators=(",", ":")), encoding="utf-8")
    current_path.write_text(json.dumps(current, sort_keys=True, separators=(",", ":")), encoding="utf-8")
    return executor(
        STATE_ROOT / f"{decision['target_digest']}.claim",
        initial_path,
        current_path,
        confirmation,
        False,
    )


def main() -> None:
    print(json.dumps(run_interactive(), sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()
