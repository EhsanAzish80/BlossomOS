#!/usr/bin/env python3
"""Privileged, bounded backend for the graphical Blossom installer."""

from __future__ import annotations

import json
import os
import stat
import sys
from pathlib import Path
from typing import Any

from scripts.distribution.installation_profile import validate_password, validate_profile
from scripts.distribution.physical_device_observer import observe, observe_ac_power
from scripts.distribution.physical_install_context import STATE_ROOT, observation
from scripts.distribution.physical_install_backend import install
from scripts.distribution.physical_install_guard import evaluate
from scripts.distribution.physical_install_harness import run_once
from scripts.distribution.provision_installed_identity import provision


MAX_INPUT_BYTES = 64 * 1024
FIELDS = {"schema", "observation", "confirmation", "profile", "password", "password_confirmation"}


def _load_request(stream: Any = sys.stdin) -> dict[str, Any]:
    payload = stream.read(MAX_INPUT_BYTES + 1)
    if len(payload.encode()) > MAX_INPUT_BYTES:
        raise ValueError("installer request exceeds the bounded size")
    value = json.loads(payload)
    if type(value) is not dict or set(value) != FIELDS or value["schema"] != 1:
        raise ValueError("installer request schema drift")
    return value


def execute(request: dict[str, Any]) -> dict[str, Any]:
    if os.geteuid() != 0:
        raise PermissionError("graphical installer backend requires root")
    profile = validate_profile(request["profile"])
    password = request["password"]
    confirmation = request["password_confirmation"]
    if not isinstance(password, str) or not isinstance(confirmation, str):
        raise ValueError("password fields must be strings")
    validate_password(password, confirmation, profile.username)
    observation_path = Path(request["observation"])
    expected_parent = Path("/run/user/1000/blossom-installer")
    if observation_path.parent != expected_parent or observation_path.name != "observation.json":
        raise ValueError("observation path is outside the installer runtime directory")
    descriptor = os.open(observation_path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        metadata = os.fstat(descriptor)
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_uid != 1000 or metadata.st_size > MAX_INPUT_BYTES:
            raise ValueError("observation file identity is invalid")
        encoded = os.read(descriptor, MAX_INPUT_BYTES + 1)
    finally:
        os.close(descriptor)
    if len(encoded) > MAX_INPUT_BYTES:
        raise ValueError("observation file exceeds the bounded size")
    initial = json.loads(encoded)
    decision = evaluate(initial, request["confirmation"])
    live, devices = observe()
    current = observation(initial["challenge"], live, devices, observe_ac_power())
    state = STATE_ROOT / f"{decision['target_digest']}.graphical.claim"

    def backend(target: dict[str, Any]) -> None:
        install(target, provision=lambda root: provision(root, profile, password, confirmation))

    result = run_once(state, initial, current, request["confirmation"], False, backend)
    return {"schema": 1, "result": result}


def main() -> None:
    request = _load_request()
    try:
        print(json.dumps(execute(request), sort_keys=True, separators=(",", ":")))
    finally:
        # Minimise secret lifetime after either success or failure.
        request["password"] = ""
        request["password_confirmation"] = ""


if __name__ == "__main__":
    main()
