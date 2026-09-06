#!/usr/bin/env python3
"""Exercise keyboard-only and compositor-close paths on the installed shell."""

import re
import subprocess
import time

import pyatspi


TIMEOUT_SECONDS = 10
ACTIVITY_PATTERN = re.compile(
    r"^Audit sequence #(\d+)  ([^\n]+)\n([^ ]+)  ·  (shell-[0-9a-f]+-\d+)$"
)


def descendants(root):
    yield root
    for index in range(root.childCount):
        try:
            yield from descendants(root.getChildAtIndex(index))
        except (LookupError, RuntimeError):
            continue


def named(name):
    return [
        node
        for node in descendants(pyatspi.Registry.getDesktop(0))
        if node.name == name
    ]


def wait_for(name):
    deadline = time.monotonic() + TIMEOUT_SECONDS
    while time.monotonic() < deadline:
        matches = named(name)
        if matches:
            return matches[0]
        time.sleep(0.1)
    raise AssertionError(f"accessible object did not appear: {name}")


def wait_absent(name):
    deadline = time.monotonic() + TIMEOUT_SECONDS
    while time.monotonic() < deadline:
        if not named(name):
            return
        time.sleep(0.1)
    raise AssertionError(f"accessible object remained visible: {name}")


def wait_focused(name):
    deadline = time.monotonic() + TIMEOUT_SECONDS
    while time.monotonic() < deadline:
        for node in named(name):
            if node.getState().contains(pyatspi.STATE_FOCUSED):
                return node
        time.sleep(0.1)
    raise AssertionError(f"accessible object did not receive focus: {name}")


def dispatch(name, argument=None):
    command = ["hyprctl", "dispatch", name]
    if argument is not None:
        command.append(argument)
    subprocess.run(command, check=True, stdout=subprocess.PIPE, text=True)


def focus_main_and_press(key):
    dispatch("focuswindow", "title:^(Blossom OS)$")
    wait_for("Request kernel identity")
    dispatch("sendshortcut", f",{key},activewindow")


def require_alert(name):
    node = wait_for(name)
    if node.getRole() != pyatspi.ROLE_ALERT:
        raise AssertionError(f"terminal state is not an accessibility alert: {name}")


def activity_for_latest_request():
    records = []
    for node in descendants(pyatspi.Registry.getDesktop(0)):
        match = ACTIVITY_PATTERN.match(node.name or "")
        if match:
            records.append(
                (int(match.group(1)), match.group(2), match.group(3), match.group(4))
            )
    if not records:
        raise AssertionError("no projected activity records found")
    latest_request = max(records)[3]
    return [record for record in records if record[3] == latest_request]


# The command bar restores focus to its request button after each terminal state.
# Return therefore starts a request without an assistive action invocation.
focus_main_and_press("return")
wait_for("Approval required")
wait_focused("Deny")
dispatch("sendshortcut", ",return,activewindow")
require_alert("Status: denied")

# Tab has a closed two-control cycle and reaches approve exactly once.
focus_main_and_press("return")
wait_for("Approval required")
wait_focused("Deny")
dispatch("sendshortcut", ",tab,activewindow")
wait_focused("Approve once")
dispatch("sendshortcut", ",return,activewindow")
require_alert("Status: verified")

# Escape is a global window shortcut and must cancel without execution.
focus_main_and_press("return")
wait_for("Approval required")
wait_focused("Deny")
dispatch("sendshortcut", ",escape,activewindow")
wait_absent("Approval required")
require_alert("Status: cancelled")
escape_records = activity_for_latest_request()
if [record[2] for record in escape_records] != [
    "accepted",
    "policy_ask",
    "approval_issued",
    "cancelled",
]:
    raise AssertionError(f"Escape path activity drift: {escape_records}")

# A compositor close request follows QML onClosing and must also cancel.
focus_main_and_press("return")
wait_for("Approval required")
wait_focused("Deny")
dispatch("closewindow", "title:^(Blossom OS approval)$")
wait_absent("Approval required")
require_alert("Status: cancelled")
close_records = activity_for_latest_request()
if [record[2] for record in close_records] != [
    "accepted",
    "policy_ask",
    "approval_issued",
    "cancelled",
]:
    raise AssertionError(f"close path activity drift: {close_records}")

print("installed keyboard and compositor-close matrix passed")
