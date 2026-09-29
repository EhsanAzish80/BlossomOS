#!/usr/bin/env python3
"""Keep the Desktop Foundation design and deliberate build lock in sync."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
plan = (ROOT / "docs/DESKTOP_FOUNDATION.md").read_text(encoding="utf-8")
normalized_plan = " ".join(plan.split())
lock = ROOT / "distribution/DESKTOP_BUILD_LOCK"
builder = (ROOT / "scripts/distribution/build_physical_candidate.sh").read_text(encoding="utf-8")
workflow = (ROOT / ".github/workflows/phase11-physical-candidate.yml").read_text(encoding="utf-8")
hardware = (ROOT / "docs/HARDWARE_SUPPORT_MATRIX.md").read_text(encoding="utf-8")
normalized_hardware = " ".join(hardware.split())
issue = (ROOT / "distribution/archiso/airootfs/etc/issue").read_text(encoding="utf-8")
motd = (ROOT / "distribution/archiso/airootfs/etc/motd").read_text(encoding="utf-8")
smoke = (ROOT / "docs/DESKTOP_GRAPHICAL_SMOKE.md").read_text(encoding="utf-8")
runtime = (ROOT / "docs/DESKTOP_RUNTIME_READINESS.md").read_text(encoding="utf-8")

required = (
    "physical candidate builds remain blocked",
    "both a normal personal computer and an agent-native computer",
    "normal path contains no ArchISO branding",
    "complete desktop appears behind a dismissible",
    "works offline",
    "non-focusable wallpaper/background layer",
    "bottom dock",
    "device unavailable, disabled, disconnected, connecting, limited and online",
    "Agent absence never blocks the ordinary desktop",
    "local graphical smoke run",
)
for statement in required:
    if statement not in normalized_plan:
        raise SystemExit(f"desktop foundation requirement missing: {statement}")

if lock.exists():
    if "DESKTOP_BUILD_LOCK" not in builder or "do not bypass this lock" not in builder or \
            '"$mode" != vm-qualification' not in builder:
        raise SystemExit("desktop build lock is not enforced by the builder")
    if "test ! -e distribution/DESKTOP_BUILD_LOCK" not in workflow:
        raise SystemExit("desktop build lock is not enforced by the workflow")
    if "Status: VM qualification gate open" not in plan:
        raise SystemExit("desktop plan status disagrees with active build lock")
elif "Status: physical build gate open" not in plan:
    raise SystemExit("removing the desktop build lock requires opening the documented gate")

for statement in (
    "absence from this table means unverified, not unsupported",
    "Adapter not yet identified",
    "base system installs from the ISO without network access",
    "Agent/model availability is reported separately",
):
    if statement not in normalized_hardware:
        raise SystemExit(f"hardware support boundary missing: {statement}")
if "Blossom OS Live" not in issue or "Arch Linux" in issue:
    raise SystemExit("live console identity is not Blossom-owned")
if "does not require an Internet connection" not in motd:
    raise SystemExit("offline installation message is missing")
for statement in (
    "Status: defined, not executed",
    "Desktop after **Continue to desktop**",
    "Installed-mode desktop with no Install action",
    "2880x1800 at scale 2",
    "Quick Settings with Wi-Fi/Ethernet",
    "Logout returns to ReGreet",
    "It does not authorize an ISO",
):
    if statement not in smoke:
        raise SystemExit(f"graphical smoke contract missing: {statement}")

for statement in (
    "Status: Linux compile qualified; graphical runtime blocked",
    "compiled successfully",
    "stale evidence",
    "No physical disk may be touched",
    "logout to ReGreet",
    "require one image built from the reviewed commit",
):
    if statement not in runtime:
        raise SystemExit(f"runtime readiness boundary missing: {statement}")

print("desktop foundation source gate verified")
