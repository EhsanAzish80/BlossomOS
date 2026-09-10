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
evidence = read("docs/PHASE_11_PHYSICAL_EVIDENCE.md")
guard = read("scripts/distribution/physical_install_guard.py")
guard_doc = read("docs/PHASE_11_INSTALL_GUARD.md")
harness = read("scripts/distribution/physical_write_harness.py")

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
for required in (
    "73659253a8d3e9e10bbb1f194e99f2f016cc4e31",
    "34447352126",
    "10140148605",
    "b53f1d14bb0b7c32fdf89fcd20f3de3bc0cc66e1d8cc0d5b8e8339eae863d41b",
    '"authority":"read_only_preflight_only"',
    '"result":"eligible_for_qualification"',
):
    if required not in evidence:
        fail(f"physical preflight evidence is incomplete: {required}")
for required in (
    "guard_decision_only",
    "guard_passed_no_write_performed",
    "exactly one unmounted internal target is required",
    "ERASE {target['path']}",
):
    if required not in guard:
        fail(f"physical install guard is incomplete: {required}")
for forbidden in ("sgdisk", "mkfs", "parted", "wipefs", "subprocess", "os.system"):
    if forbidden in guard:
        fail(f"physical install guard gained write authority: {forbidden}")
for required in ("Status: implemented", "guard_passed_no_write_performed", "once-only"):
    if required not in guard_doc:
        fail(f"physical install guard document is incomplete: {required}")
for required in ("O_EXCL", "target changed before execution", "attempt already consumed", "backend failed after once-only claim"):
    if required not in harness: fail(f"physical write harness is incomplete: {required}")
for forbidden in ("subprocess", "sgdisk", "mkfs", "wipefs"):
    if forbidden in harness: fail(f"physical write harness embeds a real writer: {forbidden}")
print("Phase 11 physical qualification boundary passed.")
