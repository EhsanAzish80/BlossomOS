#!/usr/bin/env python3
"""Keep the desktop launcher a fixed, session-local allowlist."""

from pathlib import Path
import configparser

ROOT = Path(__file__).resolve().parents[2]
LAUNCHER = ROOT / "system" / "desktop-launcher"
BUS_NAME = "org.blossomos.Desktop1"
OBJECT = "/org/blossomos/Desktop1"
BINARY = "/usr/lib/blossom-os/blossom-desktop-launcher"
UNIT = "blossom-desktop-launcher.service"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def main() -> None:
    expected = {"CMakeLists.txt", "main.cpp", UNIT, f"{BUS_NAME}.service"}
    require({path.name for path in LAUNCHER.iterdir()} == expected,
            "unexpected desktop launcher surface")

    activation = configparser.ConfigParser()
    activation.optionxform = str
    activation.read(LAUNCHER / f"{BUS_NAME}.service")
    section = activation["D-BUS Service"]
    require(section.get("Name") == BUS_NAME, "desktop activation bus name drift")
    require(section.get("Exec") == BINARY, "desktop activation executable drift")
    require(section.get("SystemdService") == UNIT, "desktop activation unit drift")

    unit = (LAUNCHER / UNIT).read_text()
    for value in [
        "PartOf=graphical-session.target",
        "Type=dbus",
        f"BusName={BUS_NAME}",
        f"ExecStart={BINARY}",
        "NoNewPrivileges=yes",
        "ProtectSystem=strict",
        "ProtectHome=read-only",
        "StateDirectory=blossom-os",
        "StateDirectoryMode=0700",
        "UMask=0077",
        "RestrictSUIDSGID=yes",
        "RestrictAddressFamilies=AF_UNIX",
    ]:
        require(value in unit, f"missing desktop launcher boundary: {value}")
    require("[Install]" not in unit, "desktop launcher must remain D-Bus activated")

    source = (LAUNCHER / "main.cpp").read_text()
    for value in [
        f'"{BUS_NAME}"',
        f'"{OBJECT}"',
        "bool Launch1(const QString &action)",
        "QByteArray QuickStatus1() const",
        "bool OnboardingRequired1() const",
        "bool CompleteOnboarding1()",
        'QStringLiteral("/onboarding-complete")',
        'marker.write("schema=1\\n")',
        'QStringLiteral("/usr/bin/hyprctl")',
        'QStringLiteral("dispatch")',
        'QStringLiteral("exec")',
        'QStringLiteral("--")',
        'QStringLiteral("/usr/bin/nmcli")',
        'QStringLiteral("/usr/bin/wpctl")',
        'QStringLiteral("/usr/bin/bluetoothctl")',
        'output.size() > 16 * 1024',
    ]:
        require(value in source, f"missing fixed desktop launcher binding: {value}")
    actions = ["terminal", "files", "browser", "editor", "network", "audio",
               "bluetooth", "audio-mute", "audio-down", "audio-up", "notifications",
               "logout", "installer", "restart", "poweroff"]
    for action in actions:
        require(source.count(f'action == QStringLiteral("{action}")') == 1,
                f"desktop action drift: {action}")
    require(source.count("QProcess::startDetached") == 1,
            "desktop launcher must have exactly one process boundary")
    require(source.count("QProcess process;") == 1 and
            "process.waitForStarted(500)" in source and "process.waitForFinished(1000)" in source,
            "desktop status probes must use one bounded process boundary")
    require("else return false;" in source,
            "unknown desktop actions must fail closed")
    require(source.count("STATE_DIRECTORY") == 2,
            "onboarding state must use only the systemd-owned state directory")
    for value in ["system(", "popen(", "/bin/sh", "sh -c", "bash -c", "QDBusMessage"]:
        require(value not in source, f"forbidden desktop launcher authority: {value}")


if __name__ == "__main__":
    main()
