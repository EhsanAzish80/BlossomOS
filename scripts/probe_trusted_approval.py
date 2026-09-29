#!/usr/bin/env python3
"""Prove that an unauthenticated model-effect approval fails closed."""

from __future__ import annotations

import argparse
import json
import math
import time

from gi.repository import Gio, GLib

BUS_NAME = "org.blossomos.Shell1"
OBJECT_PATH = "/org/blossomos/Shell1"
INTERFACE = "org.blossomos.Shell1"
PROTOCOL_VERSION = 1
ACTIVITY_LIMIT = 64


class ProbeError(RuntimeError):
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
        raise ProbeError(f"{method} failed: {error.message[:240]}") from error
    try:
        return json.loads(bytes(response.unpack()[0]))
    except (UnicodeError, json.JSONDecodeError, IndexError, TypeError) as error:
        raise ProbeError(f"{method} returned malformed JSON") from error


def read_activity(connection: Gio.DBusConnection, after: int, timeout: int) -> tuple[list[dict], int]:
    records: list[dict] = []
    cursor = after
    deadline = time.monotonic() + timeout
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise ProbeError("activity read timed out")
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
            raise ProbeError("activity response is not a list")
        for record in page:
            if record.get("sequence") != cursor + 1:
                raise ProbeError("activity sequence gap")
            cursor += 1
            records.append(record)
        if len(page) < ACTIVITY_LIMIT:
            return records, cursor


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--case", choices=("self_approval", "missing_agent"), required=True)
    args = parser.parse_args()

    connection = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    _, baseline = read_activity(connection, 0, 10)
    prompt = json.dumps(
        {
            "version": PROTOCOL_VERSION,
            "prompt": "Create qualification-note.txt containing exactly: Blossom qualification passed.",
        },
        separators=(",", ":"),
    ).encode()
    outcome = call_bytes(connection, "StartAgentTurn1", prompt, 30)
    if outcome.get("status") != "awaiting_approval" or not isinstance(outcome.get("preview"), dict):
        raise ProbeError("model turn did not produce a bounded approval preview")
    preview = outcome["preview"]
    request_id = preview.get("request_id")
    if not isinstance(request_id, str):
        raise ProbeError("preview omitted request_id")
    decision = json.dumps(
        {
            "kind": "submit_decision",
            "version": PROTOCOL_VERSION,
            "request_id": request_id,
            "preview_sha256": preview.get("preview_sha256"),
            "decision": "approve_once",
        },
        separators=(",", ":"),
    ).encode()
    try:
        call_bytes(connection, "SubmitDecision1", decision, 35)
    except ProbeError as error:
        if "AccessDenied" not in str(error):
            raise
    else:
        raise ProbeError("unauthenticated approve_once was accepted")

    activity, _ = read_activity(connection, baseline, 10)
    relevant = [record for record in activity if record.get("request_id") == request_id]
    effects = [record for record in relevant if record.get("kind") == "effect"]
    executor_starts = [
        record
        for record in relevant
        if record.get("kind") == "execution" and record.get("category") == "started"
    ]
    rejection = [
        record
        for record in relevant
        if record.get("kind") == "approval"
        and record.get("category") == "authentication_rejected"
    ]
    if effects or executor_starts or len(rejection) != 1:
        raise ProbeError(
            f"fail-closed audit mismatch: effects={len(effects)} "
            f"executor_starts={len(executor_starts)} rejections={len(rejection)}"
        )
    print(
        json.dumps(
            {
                "case": args.case,
                "status": "passed",
                "audit_reason": "authentication_rejected",
                "effects": 0,
                "executor_starts": 0,
            },
            separators=(",", ":"),
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
