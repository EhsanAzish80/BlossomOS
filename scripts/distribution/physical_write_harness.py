"""Once-only Phase 11 harness with an injected disposable-media backend."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, Callable

from scripts.distribution.physical_install_guard import evaluate


class HarnessError(RuntimeError):
    """Raised when a disposable test cannot proceed safely."""



def run_once(
    state: Path,
    initial: dict[str, Any],
    current: dict[str, Any],
    confirmation: str,
    cancelled: bool,
    backend: Callable[[dict[str, Any]], None],
) -> str:
    """Revalidate, consume once before effect, and return a truthful outcome."""
    if cancelled:
        return "cancelled_no_write"
    first = evaluate(initial, confirmation)
    if first["target"]["purpose"] != "disposable_test":
        raise HarnessError("disposable harness rejects physical-install authority")
    if first["result"] != "guard_passed_no_write_performed":
        raise HarnessError("confirmation failed")
    latest = evaluate(current, confirmation)
    if latest["target_digest"] != first["target_digest"] or latest["result"] != first["result"]:
        raise HarnessError("target changed before execution")
    state.parent.mkdir(parents=True, exist_ok=True)
    try:
        fd = os.open(state, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError as error:
        raise HarnessError("attempt already consumed") from error
    with os.fdopen(fd, "w") as stream:
        json.dump({"schema": 1, "target_digest": first["target_digest"], "state": "claimed"}, stream)
        stream.flush()
        os.fsync(stream.fileno())
    try:
        backend(first["target"])
    except Exception as error: raise HarnessError("backend failed after once-only claim") from error
    return "disposable_test_completed"
