#!/usr/bin/env python3
"""Fail-closed repository checks for the first Phase 11 boundary."""

import hashlib
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
probe = read("scripts/distribution/disposable_media_probe.py")
probe_runner = read("scripts/distribution/run_disposable_media_test.py")
install_harness = read("scripts/distribution/physical_install_harness.py")
install_runner = read("scripts/distribution/run_physical_install.py")
observer = read("scripts/distribution/physical_device_observer.py")
observation_workflow = read(".github/workflows/phase11-device-observation.yml")
disposable_evidence = read("docs/PHASE_11_DISPOSABLE_EVIDENCE.md")

evidence_hashes = {
    "distribution/evidence/phase11-device-preflight-34460218314.json": "5de11cb6ab30605ef4bcda084d2d73994a94269f3e99e9ac436f19b9b1c32c3c",
    "distribution/evidence/phase11-device-observation-34460218314.json": "805e4cabbd4f70572c103038c1e6292bd8f98694738c2ff6ac7ce527bc8c1394",
    "distribution/evidence/phase11-device-decision-34460218314.json": "13928b91fd0e6fbffd8cb8c58f65432a83fc179b6a0d4e96fb78571af6ab234b",
    "distribution/evidence/phase11-disposable-probe-result-20260910.json": "40bf0d8ae0f9b4de81d973ad2f329a9e326c74b50375bbe592684907e2b46444",
}
for path, expected in evidence_hashes.items():
    actual = hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
    if actual != expected:
        fail(f"Phase 11 disposable evidence hash drift: {path}")
for required in (
    "34460218314",
    "disposable_test_completed",
    "disposable_probe_completed_and_restored",
    "f7436d816d5efe2c8d7e8da02b31755a10e0124edfbfde09f0b8f3e96ad6f911",
):
    if required not in disposable_evidence:
        fail(f"Phase 11 disposable evidence is incomplete: {required}")

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
    '"physical_install", "disposable_test"',
    "exactly one unmounted {target_kind} target is required",
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
for required in ("O_EXCL", "target changed before execution", "attempt already consumed", "backend failed after once-only claim", "disposable harness rejects physical-install authority"):
    if required not in harness: fail(f"physical write harness is incomplete: {required}")
for forbidden in ("subprocess", "sgdisk", "mkfs", "wipefs"):
    if forbidden in harness: fail(f"physical write harness embeds a real writer: {forbidden}")
for required in ("O_EXCL", "O_NOFOLLOW", "BLKGETSIZE64", "PROBE_BYTES = 4096", "PROBE_OFFSET = 8 * 1024 * 1024", "disposable_probe_completed_and_restored"):
    if required not in probe: fail(f"disposable probe is incomplete: {required}")
for forbidden in ("subprocess", "shell=True", "os.system", "mkfs", "wipefs", "parted", "sgdisk"):
    if forbidden in probe: fail(f"disposable probe exceeds its bounded authority: {forbidden}")
for required in ("MAX_INPUT_BYTES", "run_once", "probe(target)"):
    if required not in probe_runner: fail(f"disposable probe runner is incomplete: {required}")
for required in ("/usr/bin/lsblk", "MAX_OUTPUT_BYTES", "MAX_DEVICES", '"/cdrom"'):
    if required not in observer: fail(f"physical device observer is incomplete: {required}")
for forbidden in ("sudo", "sgdisk", "mkfs", "wipefs", "parted", "dd if="):
    if forbidden in observer: fail(f"physical device observer gained write authority: {forbidden}")
for required in (
    "physical harness rejects disposable-test authority",
    "target changed before execution",
    "attempt already consumed",
    "installer failed after once-only claim",
):
    if required not in install_harness:
        fail(f"physical install harness is incomplete: {required}")
for forbidden in ("subprocess", "sgdisk", "mkfs", "wipefs", "parted"):
    if forbidden in install_harness:
        fail(f"physical install harness embeds a writer: {forbidden}")
for required in (
    "MAX_INPUT_BYTES",
    'Path("/usr/local/libexec/blossom-physical-install-backend")',
    'subprocess.run([str(BACKEND), target["path"]]',
    "timeout=1800",
):
    if required not in install_runner:
        fail(f"physical install runner is incomplete: {required}")
for forbidden in ("sgdisk", "mkfs", "wipefs", "parted", "shell=True"):
    if forbidden in install_runner:
        fail(f"physical install runner gained inline destructive authority: {forbidden}")
for required in (
    "workflow_dispatch:",
    "runs-on: [self-hosted, linux, x64, blossom-gpu]",
    "physical_preflight.py --observe-host",
    "physical_device_observer.py",
    "physical_install_guard.py",
    '"confirmation_required"',
    "phase11-device-observation",
):
    if required not in observation_workflow:
        fail(f"physical device observation workflow is incomplete: {required}")
for forbidden in ("pull_request:", "push:", "sudo", "sgdisk", "mkfs", "wipefs", "parted", "physical_write_harness"):
    if forbidden in observation_workflow:
        fail(f"physical device observation workflow gained forbidden authority: {forbidden}")
print("Phase 11 physical qualification boundary passed.")
