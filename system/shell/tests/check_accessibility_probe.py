#!/usr/bin/env python3
"""Confirm that a standard Qt window publishes controls to AT-SPI."""

import time

import pyatspi


NAME = "Standard Qt accessibility probe"
DESCRIPTION = "Confirm the standard Qt window is published to AT-SPI."
deadline = time.monotonic() + 10

while time.monotonic() < deadline:
    pending = [pyatspi.Registry.getDesktop(0)]
    while pending:
        node = pending.pop()
        if node.name == NAME:
            if node.description != DESCRIPTION:
                raise AssertionError("standard Qt accessibility description drift")
            print("standard Qt accessibility probe passed")
            raise SystemExit(0)
        for index in range(node.childCount):
            try:
                pending.append(node.getChildAtIndex(index))
            except (LookupError, RuntimeError):
                continue
    time.sleep(0.1)

raise AssertionError("standard Qt control did not appear in AT-SPI")
