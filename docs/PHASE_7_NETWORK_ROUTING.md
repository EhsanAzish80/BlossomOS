# Phase 7 network request routing and shell projection

Status: complete on 2026-09-07. Request routing, narrow shell projection, and
installed online and isolated non-internet evidence have passed. Exact offline,
local, and limited mappings remain covered by deterministic core tests.

The argument-free `system.network.connectivity` request is mapped only to
`system.read:network.connectivity`. Policy defaults to deny. An explicit allow
routes to the fixed NetworkManager provider, validates its typed observation,
and returns the closed connectivity enum without invoking the command executor.
Provider errors fail closed and are recorded without a command fallback.

The audit boundary records request identity, fixed source, schema version, and
failure category. It does not record the observed connectivity class, timestamp,
interface, address, route, DNS data, access point, or traffic.

The shell service exposes one fixed versioned `ReadNetworkConnectivity1` method.
It accepts no caller-selected provider, target, property, lifetime, polling
interval, or field. The native Qt client accepts only `offline`, `local`,
`limited`, `online`, or `unknown`, requires the exact three-field schema and
code-owned five-second lifetime, clears stale state, and fails closed on service
loss. QML receives only the verified enum and cannot issue generic D-Bus or
network operations.

Local verification passed the complete workspace tests, Clippy with warnings as
errors, repository and packaging checks, QML/client boundary checks, smoke
tests, diff hygiene, and the forbidden-name scan. Linux Quality run
[`34123160196`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34123160196)
passed at signed commit `d252f00`, including the GNU/Linux provider, production
D-Bus service, Qt client build, Rust lint, and all Rust tests.

Installed run
[`34126485600`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34126485600)
passed the real online and isolated non-internet boundaries at signed commit
`a9f934c`. Details and limits are recorded in
`docs/PHASE_7_NETWORK_INSTALLED_EVIDENCE.md`.
