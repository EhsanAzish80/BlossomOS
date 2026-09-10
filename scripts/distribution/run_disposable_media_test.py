#!/usr/bin/env python3
"""Run one confirmed Phase 11 disposable-media probe from frozen observations."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from scripts.distribution.disposable_media_probe import probe
from scripts.distribution.physical_write_harness import run_once


MAX_INPUT_BYTES = 64 * 1024


def _load(path: Path) -> dict[str, Any]:
    payload = path.read_bytes()
    if len(payload) > MAX_INPUT_BYTES:
        raise ValueError("observation exceeds the bounded size")
    value = json.loads(payload)
    if type(value) is not dict:
        raise ValueError("observation must be an object")
    return value


def execute(
    state: Path,
    initial_path: Path,
    current_path: Path,
    confirmation: str,
    cancelled: bool,
) -> dict[str, Any]:
    evidence: list[dict[str, Any]] = []
    outcome = run_once(
        state,
        _load(initial_path),
        _load(current_path),
        confirmation,
        cancelled,
        lambda target: evidence.append(probe(target)),
    )
    return {
        "schema": 1,
        "result": outcome,
        "probe": evidence[0] if evidence else None,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Run one bounded disposable-media test")
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
