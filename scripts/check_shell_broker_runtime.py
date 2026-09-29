#!/usr/bin/env python3
"""Fail closed unless the installed shell broker exposes host UID identities."""

from __future__ import annotations

import argparse
import subprocess
from pathlib import Path


class BrokerRuntimeError(RuntimeError):
    pass


def broker_pid(instance: int) -> int:
    result = subprocess.run(
        [
            "systemctl",
            "show",
            f"blossom-shell-broker@{instance}.service",
            "--property=MainPID",
            "--value",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode != 0 or not result.stdout.strip().isdigit():
        raise BrokerRuntimeError("could not resolve the shell broker PID")
    pid = int(result.stdout.strip())
    if pid <= 0:
        raise BrokerRuntimeError("shell broker is not running")
    return pid


def verify_uid_map(proc_root: Path, pid: int, expected_uid: int) -> None:
    status = (proc_root / str(pid) / "status").read_text().splitlines()
    uid_line = next((line for line in status if line.startswith("Uid:\t")), None)
    if uid_line is None:
        raise BrokerRuntimeError("shell broker status omitted Uid")
    uids = [int(value) for value in uid_line.split()[1:]]
    if uids != [expected_uid] * 4:
        raise BrokerRuntimeError("shell broker is not running entirely as the desktop user")

    lines = (proc_root / str(pid) / "uid_map").read_text().splitlines()
    fields = [line.split() for line in lines if line.strip()]
    if len(fields) != 1 or len(fields[0]) != 3:
        raise BrokerRuntimeError("shell broker UID map is not a single host identity map")
    inside, outside, length = (int(value) for value in fields[0])
    if inside != 0 or outside != 0 or length < 4_294_967_294:
        raise BrokerRuntimeError("shell broker is inside a remapping user namespace")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--instance", type=int, default=1000)
    parser.add_argument("--pid", type=int)
    parser.add_argument("--proc-root", type=Path, default=Path("/proc"))
    arguments = parser.parse_args()
    pid = arguments.pid if arguments.pid is not None else broker_pid(arguments.instance)
    verify_uid_map(arguments.proc_root, pid, arguments.instance)
    print(f"BLOSSOM_SHELL_BROKER_IDENTITY_MAP_VERIFIED pid={pid} uid={arguments.instance}")


if __name__ == "__main__":
    try:
        main()
    except (BrokerRuntimeError, OSError, ValueError) as error:
        raise SystemExit(f"shell broker runtime check failed: {error}") from error
