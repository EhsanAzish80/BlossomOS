#!/usr/bin/env python3
"""Fail-closed repository checks for the first Phase 11 boundary."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def fail(message: str) -> None:
    raise SystemExit(message)


adr = read("docs/decisions/0028-phase-11-physical-qualification.md")
baseline = read("docs/PHASE_11_BASELINE.md")
installer = read("distribution/archiso/airootfs/usr/local/bin/blossom-evidence-install")
preflight = read("scripts/distribution/physical_preflight.py")
workflow = read(".github/workflows/phase11-physical-preflight.yml")

for required in (
    "Status: Accepted",
    "MacBookPro11,1",
    "read-only preflight",
    "no installation authority",
    "typed confirmation",
):
    if required not in adr:
        fail(f"Phase 11 ADR is incomplete: {required}")
for required in ("Status: active", "physical installation remains blocked", "MacBookPro11,1"):
    if required not in baseline:
        fail(f"Phase 11 baseline is incomplete: {required}")
if "disk=/dev/vda" not in installer or "sgdisk --zap-all \"$disk\"" not in installer:
    fail("VM evidence installer identity drift")
if "read_only_preflight_only" not in preflight or "disk path" not in preflight:
    fail("physical preflight authority boundary drift")
for required in (
    "workflow_dispatch:",
    "runs-on: [self-hosted, linux, x64, blossom-gpu]",
    "physical_preflight.py --observe-host",
    "read_only_preflight_only",
):
    if required not in workflow:
        fail(f"physical preflight workflow is incomplete: {required}")
for forbidden in ("pull_request:", "sudo ", "/dev/"):
    if forbidden in workflow:
        fail(f"physical preflight workflow gained forbidden authority: {forbidden}")
print("Phase 11 physical qualification boundary passed.")
