# Phase 7 fixed NetworkManager adapter

Status: complete for Phase 7 checkpoint 9 on 2026-09-07.

The inactive production adapter contacts only the fixed system-bus destination
`org.freedesktop.NetworkManager`, object `/org/freedesktop/NetworkManager`, and
interface `org.freedesktop.NetworkManager`. It reads only `Connectivity` through
an individual `org.freedesktop.DBus.Properties.Get` call with no auto-start.

The adapter resolves the well-known owner before and after the read, requires
the unchanged owner to be UID 0, applies a two-second deadline, maps only the
five documented integer values, and validates the typed envelope before return.
Missing bus, missing owner, wrong UID, owner change, wrong type, undocumented
value, property failure, timeout, and malformed output fail closed.

It performs no interface, address, route, DNS, access-point, or traffic
enumeration and sends no external network probe.

Linux-only private-bus tests exercise fixed-property success, undocumented
values, wrong service UID, timeout, missing owner, and missing bus. Quality run
[`34121326225`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34121326225)
passed at signed commit `28e63b4`, compiling and executing the GNU/Linux path.

The adapter checkpoint itself added no public request. The separately reviewed
request route and shell projection are now complete and recorded in
`docs/PHASE_7_NETWORK_ROUTING.md`.
