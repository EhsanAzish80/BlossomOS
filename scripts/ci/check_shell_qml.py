#!/usr/bin/env python3
"""Validate that the Phase 6 QML remains a narrow presentation surface."""

from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
QML = ROOT / "system" / "shell" / "qml"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def relative_luminance(hex_color: str) -> float:
    channels = [int(hex_color[index:index + 2], 16) / 255 for index in (1, 3, 5)]
    linear = [value / 12.92 if value <= 0.04045 else ((value + 0.055) / 1.055) ** 2.4
              for value in channels]
    return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]


def contrast_ratio(first: str, second: str) -> float:
    high, low = sorted((relative_luminance(first), relative_luminance(second)), reverse=True)
    return (high + 0.05) / (low + 0.05)


def main() -> None:
    expected = {"shell.qml", "quickshell.qml", "CommandBar.qml", "SecurityHost.qml", "wallpaper.svg", "ApprovalPanel.qml", "ActivityPanel.qml", "SecurityField.qml", "BlossomButton.qml", "BlossomIconButton.qml", "BlossomDockItem.qml", "BlossomWindowDockItem.qml", "BlossomMenuItem.qml", "icon-apps.svg", "icon-files.svg", "icon-browser.svg", "icon-terminal.svg", "icon-agent.svg", "README.md", "Petal"}
    require({path.name for path in QML.iterdir()} == expected, "unexpected QML surface")
    qml = "\n".join((QML / name).read_text() for name in expected if name.endswith(".qml"))
    petal = QML / "Petal"
    petal_files = {str(path.relative_to(petal)) for path in petal.rglob("*") if path.is_file()}
    require({"qmldir", "Theme.qml"} <= petal_files, "Petal design tokens are missing")
    require(all(name in {"qmldir", "Theme.qml"} or
                (name.startswith("icons/") and name.endswith(".svg"))
                for name in petal_files), "unexpected Petal design-token surface")
    theme = (petal / "Theme.qml").read_text()
    require("pragma Singleton" in theme and "import Blossom.Shell" not in theme and
            "function " not in theme, "Petal must stay presentation-only tokens")
    qml += "\n" + theme
    colors = dict(re.findall(r"readonly property color (\w+): \"(#[0-9a-fA-F]{6})\"", theme))
    for foreground, background in (
        ("text", "base"),
        ("textSecondary", "base"),
        ("text", "surface"),
        ("textSecondary", "surface"),
        ("text", "raised"),
        ("textSecondary", "raised"),
        ("verified", "surface"),
        ("decision", "surface"),
        ("blossom", "surface"),
    ):
        require(contrast_ratio(colors[foreground], colors[background]) >= 4.5,
                f"Petal contrast below 4.5:1: {foreground} on {background}")
    require("placeholderTextColor: Theme.textSecondary" in qml,
            "command input placeholder must use a contrast-qualified token")
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
        "BlossomBroker.requestAgentTurn(agentPrompt.text)",
        "broker.approveOnce()",
        "broker.deny()",
        "broker.cancelPending()",
        "BlossomBroker.refreshActivity()",
        "BlossomBroker.openFiles()",
        "BlossomBroker.openBrowser()",
        "BlossomBroker.openEditor()",
        "BlossomBroker.openNetworkSettings()",
        "BlossomBroker.openAudioSettings()",
    ]:
        require(action in qml, f"missing closed UI action: {action}")
    for forbidden in [
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
    security_qml = "\n".join((QML / name).read_text() for name in ("SecurityHost.qml", "ApprovalPanel.qml", "SecurityField.qml"))
    require("elide:" not in security_qml,
            "approval preview security fields must never elide exact values")
    for required in [
        'Accessible.name: "Original request"',
        'Accessible.name: "Full destination"',
        'Accessible.name: "Destination folder"',
        'Accessible.name: "Content byte length"',
        'Accessible.name: "Complete proposed content"',
        'Accessible.name: "Proposal source"',
        "Text.WrapAnywhere",
        "TextEdit.WrapAnywhere",
        "broker.preview.user_request",
        "broker.preview.content",
    ]:
        require(required in security_qml,
                f"exact-effect approval disclosure missing: {required}")
    require("PanelWindow" not in security_qml,
            "security controls must not use the inaccessible proxy-window hierarchy")
    approval_qml = (QML / "ApprovalPanel.qml").read_text()
    require('text: "🔒  Blossom approval · drawn by the system broker"' in approval_qml,
            "approval must identify the fixed system-broker surface")
    require('import "Petal"' not in approval_qml and "Theme." not in approval_qml,
            "approval presentation must remain independent from the Petal theme")
    shell = (QML / "shell.qml").read_text()
    for required in (
        'WlrLayershell.namespace: "blossom-approval-trust-cue"',
        '["waiting", "submitting"].includes(BlossomBroker.state)',
        'text: "🔒  Blossom approval pending · verify the system-broker header"',
        "WlrLayer.Overlay",
        "focusable: false",
    ):
        require(required in shell,
                f"broker-backed approval trust cue missing: {required}")
    window_item = (QML / "BlossomWindowDockItem.qml").read_text()
    for required in (
        "model: ToplevelManager.toplevels",
        "toplevel.activate()",
        "toplevel.minimized = true",
        "root.dockWindowTarget.close()",
        "onMenuRequested:",
        "root.dockWindowMenuX = openWindows.x + x + width / 2",
        "root.dockWindowTarget = modelData",
    ):
        require(required in qml,
                f"dock window-management contract missing: {required}")
    require("acceptedButtons: Qt.RightButton" in window_item,
            "dock window controls must be reachable from a right-click menu")
    menu_item = (QML / "BlossomMenuItem.qml").read_text()
    require("implicitHeight: 38" in menu_item and "horizontalAlignment: Text.AlignLeft" in menu_item,
            "context menus must use compact native-style rows")
    require('text: "Close Window"' in shell and "height: 94" in shell,
            "dock window menu must remain compact")
    command_bar = (QML / "CommandBar.qml").read_text()
    theme = (QML / "Petal" / "Theme.qml").read_text()
    require("property color on" not in theme,
            "Petal token names must not be parsed as QML signal handlers")
    require(re.search(
        r'if\s*\(root\.activityVisible\)\s*root\.agentHiddenForApproval\s*=\s*true',
        shell,
    ) is not None, "approval must remember an agent panel hidden for review")
    require(re.search(
        r'root\.agentHiddenForApproval\s*=\s*false\s*root\.activityVisible\s*=\s*true',
        shell,
    ) is not None, "finished approval must reopen the previously visible agent panel")
    require(re.search(
        r'\[\s*"waiting"\s*,\s*"submitting"\s*,\s*"cancelling"\s*\]'
        r'\.includes\(BlossomBroker\.state\)',
        shell,
    ) is not None, "approval must dismiss shell popups before presenting its standard Qt window")
    require("BlossomBroker.failureReason" in shell,
            "agent failure surface must report the bounded broker reason")
    require("import Quickshell.Io" in shell and 'target: "commandbar"' in shell and
            "function toggle(): void { commandBar.toggle() }" in shell,
            "command bar must export its compositor-neutral IPC shortcut")
    require("Process" not in shell and "FileView" not in shell and "Socket" not in shell,
            "Quickshell.Io may expose only the closed command-bar IPC handler")
    for required in (
        'text: "Open an app, find a file, or ask Blossom"',
        "input.clear()",
        'BlossomBroker.queryCommandBar("")',
        "BlossomBroker.queryCommandBar(input.text)",
        "BlossomBroker.queryCommandSuggestions()",
        "BlossomBroker.activateCommandRow(id)",
        'source: "Petal/icons/search.svg"',
        "WlrKeyboardFocus.Exclusive",
        "width: Math.min(720, parent.width - 48)",
        "closeForActivation()",
        "modelData.badges || []",
    ):
        require(required in command_bar, f"command-bar presentation contract missing: {required}")
    require('text: "Ask Blossom or open something"' not in command_bar,
            "command bar must not restore the obsolete heading")
    require("selectAll()" not in command_bar,
            "command bar must not retain and select prompt history")
    require("#" not in command_bar and "Qt.rgba" not in command_bar,
            "command bar colors must come only from Petal tokens")
    require(shell.count("onClicked: commandBar.show()") == 2,
            "both Blossom flower controls must open the command bar")
    require("};" not in shell,
            "shell QML must not terminate grouped properties, child objects, or handlers with semicolons")
    for required in (
        "WlrLayer.Background",
        'WlrLayershell.namespace: "blossom-background"',
        'WlrLayershell.namespace: "blossom-top-bar"',
        'WlrLayershell.namespace: "blossom-dock"',
        'source: "wallpaper.svg"',
        'text: "Continue to desktop"',
        'text: "Activity"',
        "visible: BlossomBroker.liveEnvironment",
    ):
        require(required in shell, f"desktop foundation surface missing: {required}")
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
        'Accessible.description: "Send this request to the local model through the Blossom gateway."',
        'Accessible.description: "Open the Blossom command bar."',
        'Accessible.name: "Blossom command"',
    ]:
        require(snippet in qml, f"missing accessibility contract: {snippet}")
    for target in ["approveButton", "denyButton"]:
        require(f"KeyNavigation.tab: {target}" in qml,
                f"missing deterministic keyboard navigation to {target}")
    require("Accessible.AlertMessage" in qml,
            "authoritative outcome state must be exposed as an accessibility alert")
    require('text: "Applications"' in qml and 'text: "Files"' in qml,
            "desktop application launcher is incomplete")
    require(qml.count("Accessible.ignored: false") >= 6,
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
    security_host = (QML / "SecurityHost.qml").read_text()
    for state in ["requesting", "waiting", "submitting", "cancelling"]:
        require(f'"{state}"' in shell,
                f"request control must be disabled while {state}")
    require("Qt.callLater(securityHost.restoreRequestFocus)" in security_host,
            "command focus must wait for the approval surface to unmap")


if __name__ == "__main__":
    main()
