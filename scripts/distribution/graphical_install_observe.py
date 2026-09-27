#!/usr/bin/env python3
"""Create a short-lived, non-secret observation for the graphical installer."""

from __future__ import annotations

import json
import os
from pathlib import Path

from scripts.distribution.physical_candidate_install import _observation
from scripts.distribution.physical_device_observer import observe, observe_ac_power
from scripts.distribution.physical_install_guard import evaluate
from scripts.distribution.physical_preflight import classify, observe_host


STATE = Path(os.environ.get("XDG_RUNTIME_DIR", "/run/user/1000")) / "blossom-installer"


def main() -> None:
    preflight = classify(observe_host())
    if preflight["result"] != "eligible_for_qualification":
        raise RuntimeError("This computer is not in the qualified hardware matrix.")
    challenge = os.urandom(16).hex()
    live, devices = observe()
    observation = _observation(challenge, live, devices, observe_ac_power())
    decision = evaluate(observation)
    STATE.mkdir(mode=0o700, parents=True, exist_ok=True)
    STATE.chmod(0o700)
    path = STATE / "observation.json"
    path.write_text(json.dumps(observation, sort_keys=True, separators=(",", ":")), encoding="utf-8")
    path.chmod(0o600)
    print(json.dumps({
        "schema": 1,
        "observation": str(path),
        "target": decision["target"],
        "target_digest": decision["target_digest"],
        "expected_confirmation": decision["expected_confirmation"],
    }, sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()
