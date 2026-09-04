#!/usr/bin/env python3
"""Validate that the Phase 6 QML remains a narrow presentation surface."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
QML = ROOT / "system" / "shell" / "qml"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def main() -> None:
    expected = {"shell.qml", "ApprovalPanel.qml", "ActivityPanel.qml", "SecurityField.qml", "README.md"}
    require({path.name for path in QML.iterdir()} == expected, "unexpected QML surface")
    qml = "\n".join((QML / name).read_text() for name in expected if name.endswith(".qml"))
    for field in [
        "Operation",
        "Purpose",
        "Executable",
        "Arguments",
        "Capability",
        "Resource scope",
        "Filesystem",
        "Network",
        "Privilege",
        "Expected side effects",
        "Approval",
        "Expires at (ms)",
        "Request ID",
        "Preview SHA-256",
    ]:
        require(f'label: "{field}"' in qml, f"missing security field: {field}")
    for action in [
        "BlossomBroker.requestSystemUname()",
        "BlossomBroker.approveOnce()",
        "BlossomBroker.deny()",
        "BlossomBroker.cancelPending()",
        "BlossomBroker.refreshActivity()",
    ]:
        require(action in qml, f"missing closed UI action: {action}")
    for forbidden in [
        "Quickshell.Io",
        "Process",
        "FileView",
        "Socket",
        "Hyprland.dispatch",
        "QDBus",
        "approval_token",
        "audit_id",
        "sudo",
        "pkexec",
        "systemctl",
        "/bin/sh",
    ]:
        require(forbidden not in qml, f"forbidden QML authority: {forbidden}")
    require(qml.count('text: "Approve once"') == 1, "approve control drift")
    require("Keys.onEscapePressed" in qml, "Escape must cancel pending approval")
    require("WlrKeyboardFocus.Exclusive" in qml, "approval must request explicit keyboard focus")


if __name__ == "__main__":
    main()
