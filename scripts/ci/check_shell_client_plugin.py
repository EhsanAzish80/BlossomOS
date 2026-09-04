#!/usr/bin/env python3
"""Keep the QML client plugin narrower than the authoritative shell service."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PLUGIN = ROOT / "system" / "shell" / "client-plugin"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def main() -> None:
    expected = {"CMakeLists.txt", "README.md", "blossombroker.cpp", "blossombroker.h"}
    require({path.name for path in PLUGIN.iterdir()} == expected, "unexpected client plugin surface")
    text = "\n".join((PLUGIN / name).read_text() for name in expected)
    for fixed in [
        '"org.blossomos.Shell1"',
        '"/org/blossomos/Shell1"',
        '"StartSystemUname1"',
        '"SubmitDecision1"',
        '"CancelPending1"',
        '"ReadActivity1"',
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
    require(header.count("Q_INVOKABLE") == 5, "client invokable surface drift")


if __name__ == "__main__":
    main()
