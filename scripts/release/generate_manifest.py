#!/usr/bin/env python3
"""Create canonical beta-candidate checksums and release metadata."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
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
    parser.add_argument("--root-directory", required=True, type=Path)
    parser.add_argument("--output-directory", required=True, type=Path)
    args = parser.parse_args()

    root = args.root_directory.resolve()
    output = args.output_directory.resolve()
    try:
        output.relative_to(root)
    except ValueError:
        parser.error("output directory must be inside root directory")

    resolved = []
    for artifact in args.artifact:
        path = artifact.resolve()
        try:
            bundle_path = path.relative_to(root)
        except ValueError:
            parser.error("every artifact must be inside root directory")
        resolved.append((bundle_path, path))
    resolved.sort(key=lambda item: item[0].as_posix())
    if any(not path.is_file() or path.is_symlink() for _, path in resolved):
        parser.error("every artifact must be a regular non-symlink file")

    artifacts = [
        {
            "path": bundle_path.as_posix(),
            "sha256": sha256(path),
            "size": path.stat().st_size,
        }
        for bundle_path, path in resolved
    ]
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
        "".join(
            f"{item['sha256']}  "
            f"{Path(os.path.relpath(root / item['path'], output)).as_posix()}\n"
            for item in artifacts
        ),
        encoding="ascii",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
