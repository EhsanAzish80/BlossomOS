"""Once-only Phase 11 harness for an exact confirmed physical-install target."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, Callable

from scripts.distribution.physical_install_guard import evaluate


class InstallHarnessError(RuntimeError):
    """Raised before or after a physical installation attempt fails closed."""


def run_once(
    state: Path,
    initial: dict[str, Any],
    current: dict[str, Any],
    confirmation: str,
    cancelled: bool,
    backend: Callable[[dict[str, Any]], None],
) -> str:
    if cancelled:
        return "cancelled_no_write"
    first = evaluate(initial, confirmation)
    if first["target"]["purpose"] != "physical_install":
        raise InstallHarnessError("physical harness rejects disposable-test authority")
    if first["result"] != "guard_passed_no_write_performed":
        raise InstallHarnessError("confirmation failed")
    latest = evaluate(current, confirmation)
    if latest["target_digest"] != first["target_digest"] or latest["result"] != first["result"]:
        raise InstallHarnessError("target changed before execution")
    state.parent.mkdir(parents=True, exist_ok=True)
    try:
        fd = os.open(state, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError as error:
        raise InstallHarnessError("attempt already consumed") from error
    with os.fdopen(fd, "w") as stream:
        json.dump(
            {"schema": 1, "target_digest": first["target_digest"], "state": "claimed"},
            stream,
        )
        stream.flush()
        os.fsync(stream.fileno())
    try:
        backend(first["target"])
    except Exception as error:
        raise InstallHarnessError("installer failed after once-only claim") from error
    return "physical_install_completed"
