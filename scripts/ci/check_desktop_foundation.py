#!/usr/bin/env python3
"""Keep the Desktop Foundation design and deliberate build lock in sync."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
plan = (ROOT / "docs/DESKTOP_FOUNDATION.md").read_text(encoding="utf-8")
normalized_plan = " ".join(plan.split())
lock = ROOT / "distribution/DESKTOP_BUILD_LOCK"
builder = (ROOT / "scripts/distribution/build_physical_candidate.sh").read_text(encoding="utf-8")
workflow = (ROOT / ".github/workflows/phase11-physical-candidate.yml").read_text(encoding="utf-8")

required = (
    "candidate builds are intentionally blocked",
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
    if "DESKTOP_BUILD_LOCK" not in builder or "do not bypass this lock" not in builder:
        raise SystemExit("desktop build lock is not enforced by the builder")
    if "test ! -e distribution/DESKTOP_BUILD_LOCK" not in workflow:
        raise SystemExit("desktop build lock is not enforced by the workflow")
    if "Status: source-design gate active" not in plan:
        raise SystemExit("desktop plan status disagrees with active build lock")
elif "Status: build gate open" not in plan:
    raise SystemExit("removing the desktop build lock requires opening the documented gate")

print("desktop foundation source gate verified")
