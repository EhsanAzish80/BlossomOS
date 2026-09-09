#!/usr/bin/env python3
"""Generate a deterministic SPDX 2.3 JSON SBOM from locked Cargo metadata."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def spdx_id(name: str, version: str) -> str:
    safe = re.sub(r"[^A-Za-z0-9.-]", "-", f"{name}-{version}")
    return f"SPDXRef-Package-{safe}"


def cargo_metadata() -> dict:
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1"],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    return json.loads(result.stdout)


def created_at(epoch: int) -> str:
    return datetime.fromtimestamp(epoch, timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def generate(revision: str, epoch: int) -> dict:
    metadata = cargo_metadata()
    packages_by_id = {package["id"]: package for package in metadata["packages"]}
    packages = []
    for package in sorted(
        packages_by_id.values(), key=lambda item: (item["name"], item["version"], item["id"])
    ):
        external_refs = []
        if (package.get("source") or "").startswith("registry+"):
            external_refs.append(
                {
                    "referenceCategory": "PACKAGE-MANAGER",
                    "referenceLocator": f"pkg:cargo/{package['name']}@{package['version']}",
                    "referenceType": "purl",
                }
            )
        packages.append(
            {
                "SPDXID": spdx_id(package["name"], package["version"]),
                "copyrightText": "NOASSERTION",
                "downloadLocation": "NOASSERTION",
                "externalRefs": external_refs,
                "filesAnalyzed": False,
                "licenseConcluded": "NOASSERTION",
                "licenseDeclared": package.get("license") or "NOASSERTION",
                "name": package["name"],
                "supplier": "NOASSERTION",
                "versionInfo": package["version"],
            }
        )

    relationships = []
    for node in sorted(metadata["resolve"]["nodes"], key=lambda item: item["id"]):
        source = packages_by_id[node["id"]]
        for dependency_id in sorted(node["dependencies"]):
            dependency = packages_by_id[dependency_id]
            relationships.append(
                {
                    "relatedSpdxElement": spdx_id(dependency["name"], dependency["version"]),
                    "relationshipType": "DEPENDS_ON",
                    "spdxElementId": spdx_id(source["name"], source["version"]),
                }
            )

    identity = "\n".join(
        f"{package['name']}={package['versionInfo']}" for package in packages
    ).encode()
    graph_digest = hashlib.sha256(identity).hexdigest()
    return {
        "SPDXID": "SPDXRef-DOCUMENT",
        "creationInfo": {
            "created": created_at(epoch),
            "creators": ["Tool: BlossomOS-project-owned-sbom-1"],
        },
        "dataLicense": "CC0-1.0",
        "documentNamespace": (
            "https://github.com/EhsanAzish80/BlossomOS/sbom/"
            f"{revision}/{graph_digest}"
        ),
        "name": f"BlossomOS-{revision}",
        "packages": packages,
        "relationships": relationships,
        "spdxVersion": "SPDX-2.3",
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--revision", required=True)
    parser.add_argument("--source-date-epoch", required=True, type=int)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    document = generate(args.revision, args.source_date_epoch)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(document, indent=2, sort_keys=True, separators=(",", ": ")) + "\n",
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
