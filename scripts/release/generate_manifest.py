#!/usr/bin/env python3
"""Create canonical beta-candidate checksums and release metadata."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--revision", required=True)
    parser.add_argument("--source-date-epoch", required=True, type=int)
    parser.add_argument("--artifact", action="append", required=True, type=Path)
    parser.add_argument("--output-directory", required=True, type=Path)
    args = parser.parse_args()

    resolved = sorted((path.resolve() for path in args.artifact), key=lambda path: path.name)
    if len({path.name for path in resolved}) != len(resolved):
        parser.error("artifact basenames must be unique")
    if any(not path.is_file() or path.is_symlink() for path in resolved):
        parser.error("every artifact must be a regular non-symlink file")

    artifacts = [
        {"name": path.name, "sha256": sha256(path), "size": path.stat().st_size}
        for path in resolved
    ]
    output = args.output_directory
    output.mkdir(parents=True, exist_ok=True)
    manifest = {
        "artifacts": artifacts,
        "architecture": "x86_64",
        "boot": "uefi",
        "channel": "beta-candidate-evidence",
        "revision": args.revision,
        "schema": 1,
        "source_date_epoch": args.source_date_epoch,
    }
    (output / "release-manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True, separators=(",", ": ")) + "\n",
        encoding="utf-8",
    )
    (output / "SHA256SUMS").write_text(
        "".join(f"{item['sha256']}  {item['name']}\n" for item in artifacts),
        encoding="ascii",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
