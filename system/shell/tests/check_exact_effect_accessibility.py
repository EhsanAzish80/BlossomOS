#!/usr/bin/env python3
"""Verify AT-SPI exposes the complete maximum exact-effect preview."""

import time

import pyatspi


TIMEOUT_SECONDS = 10
NAME = "a" * 60 + ".txt"
CONTENT = "Z" * 4096
WORKSPACE = "/home/blossom/Workspace"
DESTINATION = f"{WORKSPACE}/{NAME}"
REQUEST = f"create {NAME} containing {CONTENT}"


def descendants(root):
    yield root
    for index in range(root.childCount):
        try:
            yield from descendants(root.getChildAtIndex(index))
        except (LookupError, RuntimeError):
            continue


def wait_for(name):
    deadline = time.monotonic() + TIMEOUT_SECONDS
    while time.monotonic() < deadline:
        for node in descendants(pyatspi.Registry.getDesktop(0)):
            if node.name == name:
                return node
        time.sleep(0.1)
    raise AssertionError(f"accessible object did not appear: {name}")


def require_exact(name, expected):
    actual = wait_for(name).description
    if actual != expected:
        raise AssertionError(
            f"{name} was incomplete: expected {len(expected)} characters, got {len(actual)}"
        )
    if "…" in actual or "..." in actual:
        raise AssertionError(f"{name} contained an ellipsis")


wait_for("Approval required")
require_exact("Original request", REQUEST)
require_exact("Full destination", DESTINATION)
require_exact("Destination folder", WORKSPACE)
require_exact("Content byte length", "4096 UTF-8 bytes")
require_exact("Complete proposed content", CONTENT)
require_exact("Proposal source", "Parsed directly from your request")
