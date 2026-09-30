#!/usr/bin/env python3
"""Exercise Blossom trusted approval through real PolicyKit PAM authentication."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import pty
import secrets
import select
import signal
import threading
import time

from gi.repository import Gio, GLib

BUS_NAME = "org.blossomos.Shell1"
OBJECT_PATH = "/org/blossomos/Shell1"
INTERFACE = "org.blossomos.Shell1"
PROTOCOL_VERSION = 1
ACTIVITY_LIMIT = 64
WORKSPACE = Path("/home/blossom/Workspace")


class QualificationError(RuntimeError):
    pass


def call_bytes(connection: Gio.DBusConnection, method: str, payload: bytes, timeout: int) -> dict:
    try:
        response = connection.call_sync(
            BUS_NAME,
            OBJECT_PATH,
            INTERFACE,
            method,
            GLib.Variant("(ay)", (payload,)),
            GLib.VariantType.new("(ay)"),
            Gio.DBusCallFlags.NONE,
            timeout * 1000,
            None,
        )
    except GLib.Error as error:
        raise QualificationError(f"{method} failed: {error.message[:240]}") from error
    try:
        return json.loads(bytes(response.unpack()[0]))
    except (UnicodeError, json.JSONDecodeError, IndexError, TypeError) as error:
        raise QualificationError(f"{method} returned malformed JSON") from error


def read_activity(connection: Gio.DBusConnection, after: int, timeout: int) -> tuple[list[dict], int]:
    records: list[dict] = []
    cursor = after
    deadline = time.monotonic() + timeout
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise QualificationError("activity read timed out")
        response = connection.call_sync(
            BUS_NAME,
            OBJECT_PATH,
            INTERFACE,
            "ReadActivity1",
            GLib.Variant("(qbtq)", (PROTOCOL_VERSION, True, cursor, ACTIVITY_LIMIT)),
            GLib.VariantType.new("(ay)"),
            Gio.DBusCallFlags.NONE,
            max(1, math.ceil(remaining * 1000)),
            None,
        )
        page = json.loads(bytes(response.unpack()[0]))
        if not isinstance(page, list):
            raise QualificationError("activity response is not a list")
        for record in page:
            if record.get("sequence") != cursor + 1:
                raise QualificationError("activity sequence gap")
            cursor += 1
            records.append(record)
        if len(page) < ACTIVITY_LIMIT:
            return records, cursor


def process_start_time(stat: str) -> int:
    try:
        return int(stat.rsplit(")", 1)[1].lstrip().split()[19])
    except (IndexError, ValueError) as error:
        raise QualificationError("could not parse compositor start time") from error


def compositor_identity() -> tuple[int, int]:
    matches: list[tuple[int, int]] = []
    for entry in Path("/proc").iterdir():
        if not entry.name.isdecimal():
            continue
        try:
            executable = (entry / "exe").resolve(strict=True)
            status = (entry / "status").read_text()
            real_uid = int(next(line for line in status.splitlines() if line.startswith("Uid:" )).split()[1])
            if executable != Path("/usr/bin/Hyprland") or real_uid != os.getuid():
                continue
            first = (entry / "stat").read_text()
            start_time = process_start_time(first)
            second = (entry / "stat").read_text()
            if process_start_time(second) == start_time:
                matches.append((int(entry.name), start_time))
        except (FileNotFoundError, PermissionError, StopIteration, ValueError):
            continue
    if len(matches) != 1:
        raise QualificationError(f"expected one compositor process, found {len(matches)}")
    return matches[0]


class TextAgent:
    def __init__(self, compositor: tuple[int, int], password: bytes, response: str) -> None:
        self.compositor = compositor
        self.password = password
        self.response = response
        self.prompts = 0
        self._master = -1
        self._pid = 0
        self._thread: threading.Thread | None = None
        self._stop = threading.Event()

    def start(self) -> None:
        ready_read, ready_write = os.pipe()
        os.set_inheritable(ready_write, True)
        pid, master = pty.fork()
        if pid == 0:
            os.close(ready_read)
            environment = dict(os.environ)
            environment["LC_ALL"] = "C"
            os.execvpe(
                "/usr/bin/pkttyagent",
                [
                    "pkttyagent",
                    "--process",
                    f"{self.compositor[0]},{self.compositor[1]}",
                    f"--notify-fd={ready_write}",
                ],
                environment,
            )
        os.close(ready_write)
        self._pid = pid
        self._master = master
        ready, _, _ = select.select([ready_read], [], [], 5)
        if not ready:
            self.stop()
            raise QualificationError("pkttyagent did not register within five seconds")
        os.read(ready_read, 1)
        os.close(ready_read)
        self._thread = threading.Thread(target=self._respond, daemon=True)
        self._thread.start()

    def _respond(self) -> None:
        pending = b""
        while not self._stop.is_set():
            try:
                ready, _, _ = select.select([self._master], [], [], 0.2)
                if not ready:
                    continue
                chunk = os.read(self._master, 4096)
            except OSError:
                return
            if not chunk:
                return
            pending = (pending + chunk)[-8192:]
            if b"Password:" not in pending:
                continue
            pending = b""
            self.prompts += 1
            if self.response == "expiry":
                time.sleep(31)
            if self.response == "cancel":
                os.write(self._master, b"\x03")
            elif self.response == "wrong":
                os.write(self._master, secrets.token_urlsafe(24).encode() + b"\n")
            else:
                os.write(self._master, self.password + b"\n")

    def stop(self) -> None:
        self._stop.set()
        if self._pid:
            try:
                os.kill(self._pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                os.waitpid(self._pid, 0)
            except ChildProcessError:
                pass
        if self._master >= 0:
            try:
                os.close(self._master)
            except OSError:
                pass
        if self._thread:
            self._thread.join(timeout=1)

    def __enter__(self) -> "TextAgent":
        self.start()
        return self

    def __exit__(self, *_: object) -> None:
        self.stop()


def start_request(connection: Gio.DBusConnection, name: str) -> dict:
    target = WORKSPACE / name
    target.unlink(missing_ok=True)
    payload = json.dumps(
        {"version": PROTOCOL_VERSION, "prompt": f"Create {name} containing exactly: blossom pam gate."},
        separators=(",", ":"),
    ).encode()
    outcome = call_bytes(connection, "StartAgentTurn1", payload, 30)
    if outcome.get("status") != "awaiting_approval" or not isinstance(outcome.get("preview"), dict):
        raise QualificationError("model turn did not produce an approval preview")
    return outcome["preview"]


def approve(connection: Gio.DBusConnection, preview: dict, timeout: int = 45) -> tuple[dict | None, str | None]:
    payload = json.dumps(
        {
            "kind": "submit_decision",
            "version": PROTOCOL_VERSION,
            "request_id": preview["request_id"],
            "preview_sha256": preview["preview_sha256"],
            "decision": "approve_once",
        },
        separators=(",", ":"),
    ).encode()
    try:
        return call_bytes(connection, "SubmitDecision1", payload, timeout), None
    except QualificationError as error:
        return None, str(error)


def relevant_evidence(activity: list[dict], request_ids: set[str]) -> tuple[list[dict], list[dict], list[str]]:
    relevant = [item for item in activity if item.get("request_id") in request_ids]
    effects = [item for item in relevant if item.get("kind") == "effect"]
    starts = [
        item for item in relevant
        if item.get("kind") == "execution" and item.get("category") == "started"
    ]
    reasons = [
        str(item.get("category")) for item in relevant
        if item.get("kind") == "approval" and str(item.get("category", "")).startswith("authentication_")
    ]
    return effects, starts, reasons


def append_ledger(path: Path, entry: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    try:
        os.write(descriptor, json.dumps(entry, separators=(",", ":"), sort_keys=True).encode() + b"\n")
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--case",
        choices=(
            "correct", "wrong", "cancel", "expiry", "repeated", "rate_limit",
            "cooldown_reset", "missing_agent",
        ),
        required=True,
    )
    parser.add_argument("--password-fd", type=int, required=True)
    parser.add_argument("--ledger", type=Path, required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--overlay-id", required=True)
    args = parser.parse_args()

    password = os.read(args.password_fd, 4096).rstrip(b"\n")
    os.close(args.password_fd)
    if not password or len(password) > 256:
        raise QualificationError("password pipe was empty or oversized")

    connection = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    _, baseline = read_activity(connection, 0, 10)
    compositor = compositor_identity()
    response = "cancel" if args.case == "cooldown_reset" else args.case
    iterations = 2 if args.case == "repeated" else (4 if args.case == "rate_limit" else 1)
    request_ids: set[str] = set()
    errors: list[str] = []
    started = time.monotonic()
    prompt_count = 0
    if args.case == "cooldown_reset":
        time.sleep(121)
    if args.case == "missing_agent":
        preview = start_request(connection, "pam-missing-agent-1.txt")
        request_ids.add(preview["request_id"])
        _, error = approve(connection, preview)
        if error:
            errors.append(error)
    elif args.case == "rate_limit":
        for index in range(iterations):
            with TextAgent(compositor, password, "cancel") as agent:
                preview = start_request(connection, f"pam-rate-limit-{index + 1}.txt")
                request_ids.add(preview["request_id"])
                _, error = approve(connection, preview)
                if error:
                    errors.append(error)
                prompt_count += agent.prompts
    else:
        with TextAgent(compositor, password, response) as agent:
            for index in range(iterations):
                preview = start_request(connection, f"pam-{args.case}-{index + 1}.txt")
                request_ids.add(preview["request_id"])
                _, error = approve(connection, preview)
                if error:
                    errors.append(error)
            prompt_count = agent.prompts

    activity, _ = read_activity(connection, baseline, 10)
    effects, starts, reasons = relevant_evidence(activity, request_ids)
    expected_effects = iterations if args.case in {"correct", "repeated"} else 0
    expected_prompts = 3 if args.case == "rate_limit" else (0 if args.case == "missing_agent" else iterations)
    passed = (
        len(effects) == expected_effects
        and not starts
        and prompt_count == expected_prompts
        and (not errors if expected_effects else len(errors) == iterations)
        and (
            args.case != "rate_limit"
            or "authentication_rate_limited" in reasons
        )
    )
    entry = {
        "architecture": os.uname().machine,
        "case": args.case,
        "commit": args.commit,
        "elapsed_ms": round((time.monotonic() - started) * 1000),
        "effects": len(effects),
        "errors": [hashlib.sha256(item.encode()).hexdigest() for item in errors],
        "executor_starts": len(starts),
        "kind": "trusted_approval_real_pam",
        "mode": "pkttyagent_exact_process",
        "overlay_id": args.overlay_id,
        "passed": passed,
        "prompt_count": prompt_count,
        "reasons": reasons,
        "sequence": sum(1 for _ in args.ledger.open(encoding="utf-8")) + 1 if args.ledger.exists() else 1,
        "timestamp_unix": int(time.time()),
    }
    append_ledger(args.ledger, entry)
    print(json.dumps(entry, separators=(",", ":"), sort_keys=True))
    if not passed:
        raise QualificationError("real-PAM qualification assertions failed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
