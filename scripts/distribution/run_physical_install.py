#!/usr/bin/env python3
"""Run one reviewed physical installation after exact guard revalidation."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any, Callable

from scripts.distribution.physical_install_harness import run_once


MAX_INPUT_BYTES = 64 * 1024
BACKEND = Path("/usr/local/libexec/blossom-physical-install-backend")


def _load(path: Path) -> dict[str, Any]:
    payload = path.read_bytes()
    if len(payload) > MAX_INPUT_BYTES:
        raise ValueError("observation exceeds the bounded size")
    value = json.loads(payload)
    if type(value) is not dict:
        raise ValueError("observation must be an object")
    return value


def _production_backend(target: dict[str, Any]) -> None:
    subprocess.run([str(BACKEND), target["path"]], check=True, timeout=1800)


def execute(
    state: Path,
    initial_path: Path,
    current_path: Path,
    confirmation: str,
    cancelled: bool,
    backend: Callable[[dict[str, Any]], None] = _production_backend,
) -> dict[str, Any]:
    outcome = run_once(
        state,
        _load(initial_path),
        _load(current_path),
        confirmation,
        cancelled,
        backend,
    )
    return {"schema": 1, "result": outcome}


def main() -> None:
    parser = argparse.ArgumentParser(description="Run one confirmed physical install")
    parser.add_argument("--state", required=True, type=Path)
    parser.add_argument("--initial", required=True, type=Path)
    parser.add_argument("--current", required=True, type=Path)
    parser.add_argument("--confirmation", required=True)
    parser.add_argument("--cancelled", action="store_true")
    args = parser.parse_args()
    print(
        json.dumps(
            execute(
                args.state,
                args.initial,
                args.current,
                args.confirmation,
                args.cancelled,
            ),
            sort_keys=True,
            separators=(",", ":"),
        )
    )


if __name__ == "__main__":
    main()
