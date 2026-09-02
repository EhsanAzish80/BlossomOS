#!/usr/bin/env python3
"""Dependency-free repository checks for the Phase 0 baseline."""

from __future__ import annotations

import ast
import subprocess
import sys
from pathlib import Path
from typing import Iterable


ROOT = Path(__file__).resolve().parents[2]
MAX_TRACKED_BYTES = 10 * 1024 * 1024
FORBIDDEN_SUFFIXES = {
    ".gguf",
    ".img",
    ".iso",
    ".onnx",
    ".ova",
    ".qcow2",
    ".safetensors",
    ".vdi",
    ".vmdk",
}


def tracked_files() -> Iterable[Path]:
    result = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
    )
    for raw_path in result.stdout.split(b"\0"):
        if raw_path:
            yield ROOT / raw_path.decode("utf-8")


def check_python(path: Path) -> list[str]:
    if path.suffix != ".py":
        return []
    try:
        ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    except (SyntaxError, UnicodeError) as error:
        return [f"{path.relative_to(ROOT)}: invalid Python: {error}"]
    return []


def check_file(path: Path) -> list[str]:
    relative = path.relative_to(ROOT)
    problems: list[str] = []
    if not path.is_file():
        problems.append(f"{relative}: tracked path is not a regular file")
        return problems
    size = path.stat().st_size
    if size > MAX_TRACKED_BYTES:
        problems.append(f"{relative}: tracked file is larger than 10 MiB")
    if path.suffix.lower() in FORBIDDEN_SUFFIXES:
        problems.append(f"{relative}: generated image/model artifact is tracked")
    problems.extend(check_python(path))
    return problems


def main() -> int:
    problems: list[str] = []
    for path in tracked_files():
        problems.extend(check_file(path))

    diff_check = subprocess.run(
        ["git", "diff", "--check", "--cached"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if diff_check.returncode:
        problems.append(diff_check.stdout.rstrip())

    if problems:
        print("Repository checks failed:", file=sys.stderr)
        for problem in problems:
            print(f"- {problem}", file=sys.stderr)
        return 1

    print("Repository checks passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
