#!/usr/bin/env python3
"""Keep the QML client plugin narrower than the authoritative shell service."""

from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
PLUGIN = ROOT / "system" / "shell" / "client-plugin"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def main() -> None:
    expected = {"CMakeLists.txt", "README.md", "blossombroker.cpp", "blossombroker.h", "shellmain.cpp"}
    require({path.name for path in PLUGIN.iterdir()} == expected, "unexpected client plugin surface")
    text = "\n".join((PLUGIN / name).read_text() for name in expected)
    for fixed in [
        '"org.blossomos.Shell1"',
        '"/org/blossomos/Shell1"',
        '"StartSystemUname1"',
        '"StartAgentTurn1"',
        '"QueryCommandBar1"',
        '"QueryCommandSuggestions1"',
        '"ActivateCommandRow1"',
        '"SubmitDecision1"',
        '"CancelPending1"',
        '"ReadActivity1"',
        '"ReadBatterySummary1"',
        '"ReadNetworkConnectivity1"',
        '"org.blossomos.Desktop1"',
        '"/org/blossomos/Desktop1"',
        'QStringLiteral("Launch1")',
        'QStringLiteral("QuickStatus1")',
        'constexpr quint16 ProtocolVersion = 1',
        'constexpr quint16 ActivityLimit = 64',
    ]:
        require(fixed in text, f"missing fixed client binding: {fixed}")
    for forbidden in [
        "QProcess",
        "QFile",
        "QDir",
        "QNetwork",
        "system(",
        "popen(",
        "sudo",
        "pkexec",
        "/bin/sh",
        "systemctl",
        "approval_token",
    ]:
        require(forbidden not in text, f"forbidden client authority: {forbidden}")
    header = (PLUGIN / "blossombroker.h").read_text()
    require(header.count("Q_INVOKABLE") == 30, "client invokable surface drift")
    require("Q_INVOKABLE void lockScreen();" in header and
            'launchDesktop(QStringLiteral("lock"))' in text,
            "lock request must use the fixed desktop-launcher bridge")
    require("onboardingRequired" in header and "dismissOnboarding" in header,
            "fixed onboarding surface is missing")
    client = (PLUGIN / "blossombroker.cpp").read_text()
    require(client.count("QVariant::fromValue(ProtocolVersion)") == 5,
            "all version arguments must preserve unsigned 16-bit wire type")
    require("QVariant::fromValue(ActivityLimit)" in client,
            "activity limit must preserve unsigned 16-bit wire type")
    require("m_expiryTimer.setSingleShot(true)" in client,
            "native client must use a one-shot approval expiry timer")
    require("QTimer::timeout" in client and "&BlossomBroker::cancelPending" in client,
            "approval expiry must request backend-enforced cancellation")
    require('QStringLiteral("expires_at_ms")' in client,
            "approval expiry must use the fixed preview deadline")
    require("MaxApprovalDelayMs" in client,
            "approval expiry timer must reject an unbounded deadline")
    require("MaxBatteryLifetimeMs" in client and "object.size() == (present ? 5 : 3)" in client,
            "battery projection must enforce fixed lifetime and exact schema")
    require("&BlossomBroker::refreshBattery" in client,
            "battery projection must refresh only on its code-owned expiry timer")
    require("MaxNetworkLifetimeMs" in client and "object.size() != 3" in client,
            "network projection must enforce fixed lifetime and exact schema")
    require("&BlossomBroker::refreshNetwork" in client,
            "network projection must refresh only on its code-owned expiry timer")
    require("m_quickStatusTimer.setInterval(5000)" in client and
            "object.size() != 5" in client and "volume < -1 || volume > 150" in client,
            "desktop quick status must be bounded and exact-schema validated")
    require("QDBusServiceWatcher::WatchForUnregistration" in client,
            "native client must watch for loss of the fixed service owner")
    require("QDBusServiceWatcher::serviceUnregistered" in client,
            "service owner loss must trigger a fail-closed client transition")
    require("++m_serviceGeneration" in client and client.count("generation != m_serviceGeneration") == 7,
            "all asynchronous replies must fail closed after service owner loss")
    require("MaxAgentPromptBytes" in client and "prompt.toUtf8()" in client and
            "prompt.contains(QChar::Null)" in client,
            "agent prompt must be UTF-8 bounded before crossing D-Bus")
    require("MaxCommandRows" in client and "validRowId" in client and
            "safeDisplayText" in client and "m_commandGeneration" in client,
            "command rows must be bounded, opaque, and stale-reply safe")
    require('setState(QStringLiteral("requesting"))' in client,
            "request start must close the rapid-click race")
    require("MaxFailureReasonCharacters" in client and "boundedFailureReason" in client,
            "client-visible failure reasons must be bounded and stripped of controls")
    require("decisionFailureReason(reply.error())" in client and
            'error.type() == QDBusError::AccessDenied' in client and
            "emit failureReasonChanged()" in client,
            "decision failures must expose a bounded, classified reason")
    require(re.search(
        r"failClosed\s*\(\s*decisionFailureReason\s*\(\s*reply\.error\(\)\s*\)\s*\)\s*;"
        r"\s*refreshActivity\s*\(\s*\)\s*;",
        client,
    ) is not None,
            "decision failure must refresh authoritative activity")
    require('if (m_state == QStringLiteral("unavailable"))' not in client,
            "activity refresh must not erase a visible fail-closed outcome")
    cmake = (PLUGIN / "CMakeLists.txt").read_text()
    for setting in [
        "if(COMMAND qt_policy)",
        "qt_policy(SET QTP0001 NEW)",
        "set_target_properties(blossom-shell-client-plugin blossom-shell-client-pluginplugin PROPERTIES",
        'LIBRARY_OUTPUT_DIRECTORY "${QML_OUTPUT_DIRECTORY}/Blossom/Shell"',
        "BUILD_WITH_INSTALL_RPATH TRUE",
        'INSTALL_RPATH "$ORIGIN"',
    ]:
        require(setting in cmake, f"missing relocatable plugin packaging: {setting}")
    host = (PLUGIN / "shellmain.cpp").read_text()
    require('setDesktopFileName(QStringLiteral("org.blossomos.ShellApproval"))' in host,
            "approval host must expose the fixed labwc application identity")
    require('file:///usr/share/blossom-os/shell/SecurityHost.qml' in host,
            "security host must load only the fixed installed accessibility entrypoint")
    require("QApplication application" in host,
            "UI host must provide the standard Qt accessibility runtime")
    require("argc" in host and "argv" in host,
            "UI host must initialize Qt from the process arguments")
    bus_test = (ROOT / "system" / "shell" / "tests" / "client_bus.cpp").read_text()
    require('requestAgentTurn(QStringLiteral("fixture qualification"))' in bus_test and
            'broker.state() == "unavailable"' in bus_test,
            "real client bus test must exercise the agent method and fail closed without a model")
    require('queryCommandBar(QStringLiteral("fixture"))' in bus_test and
            "queryCommandSuggestions()" in bus_test and
            'QStringLiteral("ask_blossom")' in bus_test,
            "real client bus test must validate broker-authored opaque command rows")


if __name__ == "__main__":
    main()
