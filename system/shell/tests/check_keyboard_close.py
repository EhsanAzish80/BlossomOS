#!/usr/bin/env python3
"""Exercise keyboard-only and compositor-close paths on the installed shell."""

import json
import os
import re
import subprocess
import sys
import time

import pyatspi


TIMEOUT_SECONDS = 10
ACTIVITY_PATTERN = re.compile(
    r"^Audit sequence #(\d+)  ([^\n]+)\n([^ ]+)  ·  (shell-[0-9a-f]+-\d+)$"
)


def descendants(root):
    if root is None:
        return
    yield root
    try:
        child_count = root.childCount
    except (AttributeError, LookupError, RuntimeError):
        return
    for index in range(child_count):
        try:
            yield from descendants(root.getChildAtIndex(index))
        except (LookupError, RuntimeError):
            continue


def named(name):
    matches = []
    for node in descendants(pyatspi.Registry.getDesktop(0)):
        try:
            if node.name == name:
                matches.append(node)
        except (AttributeError, LookupError, RuntimeError):
            continue
    return matches


def wait_for(name):
    deadline = time.monotonic() + TIMEOUT_SECONDS
    while time.monotonic() < deadline:
        matches = named(name)
        if matches:
            return matches[0]
        time.sleep(0.1)
    raise AssertionError(f"accessible object did not appear: {name}")


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


def wait_compositor_window(title):
    deadline = time.monotonic() + TIMEOUT_SECONDS
    while time.monotonic() < deadline:
        result = subprocess.run(
            ["hyprctl", "clients", "-j"],
            check=True,
            stdout=subprocess.PIPE,
            text=True,
        )
        if any(client.get("title") == title for client in json.loads(result.stdout)):
            return
        time.sleep(0.1)
    raise AssertionError(f"compositor window did not appear: {title}")


def wait_compositor_window_absent(title):
    deadline = time.monotonic() + TIMEOUT_SECONDS
    while time.monotonic() < deadline:
        result = subprocess.run(
            ["hyprctl", "clients", "-j"],
            check=True,
            stdout=subprocess.PIPE,
            text=True,
        )
        if not any(client.get("title") == title for client in json.loads(result.stdout)):
            return
        time.sleep(0.1)
    raise AssertionError(f"compositor window remained visible: {title}")


def press(key):
    instances = json.loads(
        subprocess.run(
            ["hyprctl", "instances", "-j"],
            check=True,
            stdout=subprocess.PIPE,
            text=True,
        ).stdout
    )
    signature = os.environ["HYPRLAND_INSTANCE_SIGNATURE"]
    instance = next(item for item in instances if item["instance"] == signature)
    environment = os.environ.copy()
    environment["WAYLAND_DISPLAY"] = instance["wl_socket"]
    subprocess.run(["wtype", "-k", key], check=True, env=environment)


def focus_main_and_press(key):
    wait_compositor_window("Blossom OS")
    dispatch("focuswindow", "title:^(Blossom OS)$")
    wait_focused("Request kernel identity")
    press(key)


def focus_approval():
    wait_for("Approval required")
    wait_compositor_window("Blossom OS approval")
    dispatch("focuswindow", "title:^(Blossom OS approval)$")
    wait_focused("Deny")


def require_alert(name):
    node = wait_for(name)
    if node.getRole() != pyatspi.ROLE_ALERT:
        raise AssertionError(f"terminal state is not an accessibility alert: {name}")


def invoke(name):
    node = wait_for(name)
    action = node.queryAction()
    if action.nActions < 1 or not action.doAction(0):
        raise AssertionError(f"accessible action failed: {name}")


def activity_for_latest_request():
    records = []
    for node in descendants(pyatspi.Registry.getDesktop(0)):
        try:
            match = ACTIVITY_PATTERN.match(node.name or "")
        except (AttributeError, LookupError, RuntimeError):
            continue
        if match:
            records.append(
                (int(match.group(1)), match.group(2), match.group(3), match.group(4))
            )
    if not records:
        raise AssertionError("no projected activity records found")
    latest_request = max(records)[3]
    return sorted(record for record in records if record[3] == latest_request)


def wait_for_latest_activity(expected_outcomes):
    deadline = time.monotonic() + TIMEOUT_SECONDS
    latest_records = []
    while time.monotonic() < deadline:
        latest_records = activity_for_latest_request()
        if [record[2] for record in latest_records] == expected_outcomes:
            return latest_records
        time.sleep(0.1)
    raise AssertionError(f"activity projection drift: {latest_records}")


scenario = sys.argv[1] if len(sys.argv) == 2 else ""

if scenario == "cancel-and-deny":
    # A compositor close request follows QML onClosing and must cancel without
    # depending on keyboard focus inherited from an earlier scenario.
    invoke("Request kernel identity")
    focus_approval()
    dispatch("closewindow", "title:^(Blossom OS approval)$")
    wait_compositor_window_absent("Blossom OS approval")
    require_alert("Status: cancelled")
    wait_for_latest_activity([
        "accepted",
        "policy_ask",
        "approval_issued",
        "cancelled",
    ])

    # Escape is a global window shortcut and must cancel without execution.
    focus_main_and_press("space")
    focus_approval()
    press("Escape")
    wait_compositor_window_absent("Blossom OS approval")
    require_alert("Status: cancelled")
    wait_for_latest_activity([
        "accepted",
        "policy_ask",
        "approval_issued",
        "cancelled",
    ])

    # The command bar restores focus to its request button after each terminal
    # state. Space therefore starts and denies without an assistive invocation.
    focus_main_and_press("space")
    focus_approval()
    press("space")
    require_alert("Status: denied")
    print("installed compositor-close, Escape, and keyboard-deny paths passed")
elif scenario == "approve":
    # Start this scenario in a fresh AT-SPI client process so accessibility
    # objects destroyed by the three prior modal ceremonies cannot be reused.
    # Tab has a closed two-control cycle and reaches approve exactly once.
    focus_main_and_press("space")
    focus_approval()
    press("Tab")
    wait_focused("Approve once")
    press("space")
    require_alert("Status: verified")
    print("installed keyboard-approve path passed")
else:
    raise SystemExit("expected cancel-and-deny or approve")
