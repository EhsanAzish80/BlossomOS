#!/usr/bin/env python3
"""Keep the QML client plugin narrower than the authoritative shell service."""

from pathlib import Path

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
        '"SubmitDecision1"',
        '"CancelPending1"',
        '"ReadActivity1"',
        '"ReadBatterySummary1"',
        '"ReadNetworkConnectivity1"',
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
    require(header.count("Q_INVOKABLE") == 7, "client invokable surface drift")
    client = (PLUGIN / "blossombroker.cpp").read_text()
    require(client.count("QVariant::fromValue(ProtocolVersion)") == 4,
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
    require("QDBusServiceWatcher::WatchForUnregistration" in client,
            "native client must watch for loss of the fixed service owner")
    require("QDBusServiceWatcher::serviceUnregistered" in client,
            "service owner loss must trigger a fail-closed client transition")
    require("++m_serviceGeneration" in client and client.count("generation != m_serviceGeneration") == 6,
            "all asynchronous replies must fail closed after service owner loss")
    require('setState(QStringLiteral("requesting"))' in client,
            "request start must close the rapid-click race")
    cmake = (PLUGIN / "CMakeLists.txt").read_text()
    for setting in [
        "set_target_properties(blossom-shell-client-plugin blossom-shell-client-pluginplugin PROPERTIES",
        'LIBRARY_OUTPUT_DIRECTORY "${QML_OUTPUT_DIRECTORY}/Blossom/Shell"',
        "BUILD_WITH_INSTALL_RPATH TRUE",
        'INSTALL_RPATH "$ORIGIN"',
    ]:
        require(setting in cmake, f"missing relocatable plugin packaging: {setting}")
    host = (PLUGIN / "shellmain.cpp").read_text()
    require('file:///usr/share/blossom-os/shell/shell.qml' in host,
            "UI host must load only the fixed installed QML entrypoint")
    require("QApplication application" in host,
            "UI host must provide the standard Qt accessibility runtime")
    require("argc" in host and "argv" in host,
            "UI host must initialize Qt from the process arguments")


if __name__ == "__main__":
    main()
