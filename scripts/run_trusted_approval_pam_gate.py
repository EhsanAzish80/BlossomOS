#!/usr/bin/env python3
"""Run the disposable-overlay trusted-approval PAM qualification cases."""

from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path
import secrets
import subprocess
import time

CASES = ("correct", "wrong", "cancel", "expiry", "repeated", "rate_limit", "missing_agent")
USER = "blossom"
UID = 1000
RUNTIME = f"/run/user/{UID}"


def run(command: list[str], **kwargs: object) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(command, check=True, **kwargs)


def user_systemctl(*arguments: str, check: bool = True) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        [
            "runuser", "-u", USER, "--", "env",
            f"HOME=/home/{USER}", f"XDG_RUNTIME_DIR={RUNTIME}",
            f"DBUS_SESSION_BUS_ADDRESS=unix:path={RUNTIME}/bus",
            "systemctl", "--user", *arguments,
        ],
        check=check,
    )


def invoke_case(case: str, password: bytes, ledger: Path, commit: str, overlay_id: str) -> None:
    read_fd, write_fd = os.pipe()
    try:
        os.write(write_fd, password + b"\n")
    finally:
        os.close(write_fd)
    command = [
        "runuser", "-u", USER, "--", "env",
        f"HOME=/home/{USER}", f"XDG_RUNTIME_DIR={RUNTIME}",
        f"DBUS_SESSION_BUS_ADDRESS=unix:path={RUNTIME}/bus",
        "/usr/lib/blossom-os/qualify-trusted-approval-pam",
        "--case", case,
        "--password-fd", str(read_fd),
        "--ledger", str(ledger),
        "--commit", commit,
        "--overlay-id", overlay_id,
    ]
    try:
        subprocess.run(command, check=True, pass_fds=(read_fd,))
    finally:
        os.close(read_fd)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--commit", required=True)
    parser.add_argument("--ledger", type=Path, required=True)
    parser.add_argument("--overlay-id")
    args = parser.parse_args()
    if os.geteuid() != 0:
        raise SystemExit("this disposable-overlay orchestrator must run as root")

    ledger = args.ledger.resolve()
    qualification_home = Path(f"/home/{USER}").resolve()
    if qualification_home not in ledger.parents:
        raise SystemExit("qualification ledger must remain inside the disposable user's home")

    machine_id = Path("/etc/machine-id").read_text().strip()
    boot_id = Path("/proc/sys/kernel/random/boot_id").read_text().strip()
    overlay_id = args.overlay_id or hashlib.sha256(f"{machine_id}:{boot_id}".encode()).hexdigest()[:16]
    ledger.parent.mkdir(parents=True, exist_ok=True)
    os.chown(ledger.parent, UID, UID)
    ledger.touch(mode=0o600, exist_ok=True)
    os.chown(ledger, UID, UID)

    password = secrets.token_urlsafe(32).encode()
    run(["chpasswd"], input=USER.encode() + b":" + password + b"\n")
    try:
        for case in CASES:
            run(["faillock", "--user", USER, "--reset"])
            run(["systemctl", "restart", f"blossom-shell-broker@{UID}.service"])
            time.sleep(1)
            if case == "missing_agent":
                user_systemctl("stop", "blossom-polkit-agent.service", check=False)
                subprocess.run(
                    ["pkill", "-u", USER, "-f", "^/usr/bin/lxqt-policykit-agent$"],
                    check=False,
                )
                remaining = subprocess.run(
                    ["pgrep", "-u", USER, "-f", "^/usr/bin/lxqt-policykit-agent$"],
                    check=False,
                    stdout=subprocess.DEVNULL,
                )
                if remaining.returncode == 0:
                    raise RuntimeError("graphical PolicyKit agent remained active")
            else:
                user_systemctl("start", "blossom-polkit-agent.service")
            try:
                invoke_case(case, password, ledger, args.commit, overlay_id)
            finally:
                if case == "missing_agent":
                    user_systemctl("start", "blossom-polkit-agent.service")
    finally:
        password = b""
        run(["passwd", "--lock", USER])
        run(["faillock", "--user", USER, "--reset"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
