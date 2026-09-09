#!/usr/bin/env python3
"""Fail-closed static checks for the Phase 10 beta-candidate boundary."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
ATTEST_SHA = "508db95dd578ae2727ebd6217d5ba78e4fbda05d"
UPLOAD_SHA = "043fb46d1a93c77aae656e7c1c64a875d1fc6a0a"


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def main() -> int:
    problems: list[str] = []
    candidate = read(".github/workflows/phase10-beta-candidate.yml")
    security = read(".github/workflows/phase10-security.yml")
    limitations = read("docs/PHASE_10_LIMITATIONS.md")
    release_policy = read("docs/BRANCH_RELEASE_POLICY.md")
    security_policy = read("SECURITY.md")

    required_candidate = (
        "cmp ",
        "generate_sbom.py",
        "generate_manifest.py",
        "SOURCE_DATE_EPOCH",
        f"actions/attest@{ATTEST_SHA}",
        f"actions/upload-artifact@{UPLOAD_SHA}",
        "github.repository == 'EhsanAzish80/BlossomOS'",
    )
    for marker in required_candidate:
        if marker not in candidate:
            problems.append(f"candidate workflow missing: {marker}")

    if re.search(r"\bpull_request\s*:", candidate):
        problems.append("candidate workflow must never run pull-request code")
    if any(token in candidate for token in ("gh release", "create-release", "git tag")):
        problems.append("candidate workflow must not publish a release or tag")
    if "cargo-audit --version 0.22.2" not in security:
        problems.append("security workflow must pin cargo-audit 0.22.2")
    if "cargo-fuzz --version 0.13.2" not in security:
        problems.append("security workflow must pin cargo-fuzz 0.13.2")
    if "nightly-2026-09-01" not in security:
        problems.append("security workflow must pin the fuzz toolchain date")
    if "No version currently receives security updates" not in security_policy:
        problems.append("security policy must explicitly state support status")
    if "not constitute hardware compatibility" not in limitations:
        problems.append("limitations must distinguish runner and hardware evidence")
    if "explicit maintainer approval" not in release_policy:
        problems.append("release policy must require action-time publication approval")

    for workflow in ROOT.glob(".github/workflows/*.yml"):
        for line_number, line in enumerate(workflow.read_text().splitlines(), 1):
            if "uses:" in line and not re.search(r"@[0-9a-f]{40}(?:\s|$)", line):
                problems.append(
                    f"{workflow.relative_to(ROOT)}:{line_number}: action is not SHA-pinned"
                )

    if problems:
        print("Phase 10 checks failed:", file=sys.stderr)
        for problem in problems:
            print(f"- {problem}", file=sys.stderr)
        return 1
    print("Phase 10 boundary checks passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
