#!/usr/bin/env python3
"""Exercise the installed fixed shell flow only through AT-SPI actions."""

import time

import pyatspi


TIMEOUT_SECONDS = 10
REQUIRED_PREVIEW_FIELDS = {
    "Operation",
    "Purpose",
    "Executable",
    "Arguments",
    "Capability",
    "Resource scope",
    "Filesystem",
    "Network",
    "Privilege",
    "Expected side effects",
    "Approval",
    "Expires at (ms)",
    "Request ID",
    "Preview SHA-256",
}


def descendants(root):
    yield root
    for index in range(root.childCount):
        try:
            yield from descendants(root.getChildAtIndex(index))
        except (LookupError, RuntimeError):
            continue


def named(name):
    return [node for node in descendants(pyatspi.Registry.getDesktop(0)) if node.name == name]


def wait_for(name):
    deadline = time.monotonic() + TIMEOUT_SECONDS
    while time.monotonic() < deadline:
        matches = named(name)
        if matches:
            return matches[0]
        time.sleep(0.1)
    observed = sorted(
        {
            node.name
            for node in descendants(pyatspi.Registry.getDesktop(0))
            if node.name
        }
    )
    raise AssertionError(
        f"accessible object did not appear: {name}; observed names: {observed}"
    )


def invoke(node):
    action = node.queryAction()
    if action.nActions < 1 or not action.doAction(0):
        raise AssertionError(f"accessible action failed: {node.name}")


request = wait_for("Request kernel identity")
if request.description != "Request the fixed kernel identity diagnostic.":
    raise AssertionError("request control description drift")
invoke(request)

wait_for("Approval required")
deny = wait_for("Deny")
approve = wait_for("Approve once")
if deny.description != "Deny this request without starting execution.":
    raise AssertionError("denial description drift")
if approve.description != "Approve only this exact request for one execution.":
    raise AssertionError("approval description drift")
if not deny.getState().contains(pyatspi.STATE_FOCUSED):
    raise AssertionError("safe denial control did not receive initial focus")

field_nodes = {
    node.name: node
    for node in descendants(pyatspi.Registry.getDesktop(0))
    if node.name in REQUIRED_PREVIEW_FIELDS
}
missing = REQUIRED_PREVIEW_FIELDS - set(field_nodes)
if missing:
    raise AssertionError(f"missing accessible security fields: {sorted(missing)}")
empty_descriptions = sorted(
    name for name, node in field_nodes.items() if not node.description
)
if empty_descriptions:
    raise AssertionError(
        f"security fields missing accessible values: {empty_descriptions}"
    )

invoke(deny)
wait_for("Status: denied")
