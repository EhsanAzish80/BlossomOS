# Phase 7 battery shell projection

Status: implemented and regression-tested as Phase 7 checkpoint 5 on
2026-09-07. Real battery-present and valid no-battery evidence remain
checkpoint 6.

The shell now receives one fixed, read-only battery projection through
`ReadBatterySummary1`. The same-user session-bus service remains authoritative:
it reads the fixed UPower display device, evaluates the singleton
`system.read:battery.summary` capability, verifies freshness and schema, and
records content-minimized audit events before returning any display data.

The production shell policy deliberately allows only this low-content battery
summary. Default policy remains deny, kernel identity remains approval-gated,
and no other context source is enabled.

## Closed presentation schema

The version-1 response contains only:

- `status`: `present` or `absent`
- `percentage` and the closed battery `state` when present
- `expires_at_ms`, derived from the code-owned five-second lifetime

The native Qt client rejects oversized, malformed, stale, future, incorrectly
typed, or field-expanded responses. It refreshes only when the fixed lifetime
expires and clears the projection on service loss or any failed refresh.

QML receives one `battery` property and can request only `refreshBattery()`.
It has no generic D-Bus method, registry discovery, source selection, polling
interval, commands, files, sockets, tokens, or raw provider response.

## Evidence

- focused shell and projection tests pass, including policy routing,
  verification, caching at the code-owned minimum interval, exact output, and
  content-free activity projection;
- repository, QML, and native-client surface guards pass;
- the forbidden legacy-name scan is clean.
- Quality run `34112781806` passed on exact implementation commit `3186adc`,
  including the native Qt plugin build, repository guards, strict Clippy, and
  the complete Rust test suite;
- installed shell regression run `34112782428` passed on the same commit using
  the `blossom-x64-phase6` Intel Linux runner, including target Arch setup,
  Rust/Qt compilation, the real Qt-to-Rust session-bus test, Bubblewrap,
  package installation, and nested Hyprland loading.

Installed battery-present evidence and installed no-battery evidence are
intentionally deferred to checkpoint 6 and must pass before the first battery
slice exits.
