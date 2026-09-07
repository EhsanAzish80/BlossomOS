#!/usr/bin/env python3
"""Validate that the Phase 6 QML remains a narrow presentation surface."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
QML = ROOT / "system" / "shell" / "qml"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def main() -> None:
    expected = {"shell.qml", "quickshell.qml", "ApprovalPanel.qml", "ActivityPanel.qml", "SecurityField.qml", "README.md"}
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
    require('sequence: "Escape"' in qml, "approval must provide a window Escape shortcut")
    require("autoRepeat: false" in qml, "Escape cancellation must not auto-repeat")
    require("denyButton.forceActiveFocus(Qt.ActiveWindowFocusReason)" in qml,
            "approval must acquire the safe decision control as its focus target")
    require("requestActivate()" in qml, "approval window must request activation when shown")
    require("Qt.ApplicationModal" in qml, "approval must request application-modal keyboard focus")
    require("onClosing:" in qml,
            "standard Qt window close must cancel pending approval")
    require("PanelWindow" not in qml,
            "security controls must not use the inaccessible proxy-window hierarchy")
    require('ShellRoot {}' in (QML / "quickshell.qml").read_text(),
            "the pinned Quickshell runtime must remain independently loadable")
    require("Accessible.defaultButton: true" in qml,
            "denial must be the accessible default action")
    require(qml.count("Accessible.onPressAction") == 2,
            "both approval actions must support assistive activation")
    for snippet in [
        'Accessible.name: "Approval required"',
        'Accessible.description: "Deny this request without starting execution."',
        'Accessible.description: "Approve only this exact request for one execution."',
        'Accessible.description: "Request the fixed kernel identity diagnostic."',
        'Accessible.description: "Refresh the bounded authoritative activity list."',
    ]:
        require(snippet in qml, f"missing accessibility contract: {snippet}")
    for target in ["approveButton", "denyButton", "refreshButton", "requestButton"]:
        require(f"KeyNavigation.tab: {target}" in qml,
                f"missing deterministic keyboard navigation to {target}")
    require("Accessible.AlertMessage" in qml,
            "authoritative outcome state must be exposed as an accessibility alert")
    require('Accessible.name: "Blossom OS controls"' in qml,
            "command controls must have an accessible grouping root")
    require(qml.count("Accessible.ignored: false") >= 8,
            "interactive and security content must remain in the accessibility tree")
    accessibility_test = (ROOT / "system" / "shell" / "tests" / "check_accessibility.py").read_text()
    for required in [
        'wait_for("Request kernel identity")',
        'wait_for("Approval required")',
        'wait_for("Deny")',
        'wait_for("Approve once")',
        "STATE_FOCUSED",
        "REQUIRED_PREVIEW_FIELDS",
        'require_alert("Status: denied")',
        'require_alert("Status: verified")',
        'wait_hidden("Approval required")',
        "STATE_ENABLED",
    ]:
        require(required in accessibility_test,
                f"missing installed accessibility assertion: {required}")
    probe_qml = (ROOT / "system" / "shell" / "tests" / "accessibility_probe.qml").read_text()
    probe_test = (ROOT / "system" / "shell" / "tests" / "check_accessibility_probe.py").read_text()
    require('text: "Standard Qt accessibility probe"' in probe_qml,
            "standard Qt accessibility control is missing")
    require("standard Qt accessibility probe passed" in probe_test,
            "standard Qt AT-SPI probe assertion is missing")
    workflow = (ROOT / ".github" / "workflows" / "phase6-installed-evidence.yml").read_text()
    require('pyatspi.Registry.getDesktop(0)' in workflow,
            "installed evidence must start AT-SPI before launching Quickshell")
    require('status org.a11y.Bus' in workflow,
            "installed evidence must verify the accessibility bus is available")
    require('check_accessibility_probe.py' in workflow,
            "installed evidence must isolate standard Qt from the shell runtime")
    require('/usr/lib/blossom-os/blossom-shell-ui' in workflow,
            "installed evidence must launch the dedicated accessible UI host")
    shell = (QML / "shell.qml").read_text()
    for state in ["requesting", "waiting", "submitting", "cancelling"]:
        require(f'BlossomBroker.state !== "{state}"' in shell,
                f"request control must be disabled while {state}")
    require("Qt.callLater(commandBar.restoreRequestFocus)" in shell,
            "command focus must wait for the approval surface to unmap")


if __name__ == "__main__":
    main()
