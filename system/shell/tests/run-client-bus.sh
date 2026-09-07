#!/bin/bash
# Trusted test binaries on a disposable session bus; never sends approval.
set -euo pipefail
[[ $# == 2 && -x $1 && -x $2 && -n ${DBUS_SESSION_BUS_ADDRESS:-} ]]
"$1" &
service_pid=$!
trap 'kill "$service_pid" 2>/dev/null || true; wait "$service_pid" 2>/dev/null || true' EXIT
for attempt in {1..50}; do
    kill -0 "$service_pid"
    if busctl --user --quiet status org.blossomos.Shell1 >/dev/null 2>&1; then
        "$2"
        exit 0
    fi
    sleep 0.1
done
echo 'Test service failed to acquire its bus name.' >&2
exit 1
