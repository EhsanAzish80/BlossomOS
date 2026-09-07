# Phase 7 fixed UPower battery adapter

Status: Phase 7 checkpoint 3 complete on 2026-09-07. This checkpoint adds and
tests the fixed native observation adapter. It does not expose the adapter to
the shell or model and does not add a production policy grant.

## Fixed boundary

The Linux adapter connects only to the system bus and reads the fixed UPower
display device:

- destination `org.freedesktop.UPower`;
- object `/org/freedesktop/UPower/devices/DisplayDevice`;
- interface `org.freedesktop.UPower.Device`; and
- properties `Type`, `State`, `Percentage`, and `IsPresent` through individual
  `org.freedesktop.DBus.Properties.Get` calls.

It resolves and verifies the service's unique owner and expected UID before
reading, checks the owner again afterward, disables property caching, and
rejects replacement during the read. The complete operation has a two-second
deadline. Unsupported platforms, an unavailable bus or owner, an untrusted or
changed owner, failed properties, timeout, and malformed values fail closed.

The adapter returns only the typed present or absent value defined by ADR-0022.
It exposes no device path, hardware identity, raw property map, history, or
generic D-Bus access.

## Linux evidence

The authoritative corrected GitHub Actions Quality run is
[`34107682577`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34107682577)
(job `101696368927`). It completed successfully in 2 minutes 13 seconds on the
project's self-hosted Linux x64 evidence runner.

The run passed repository checks, packaging validation, QML validation, smoke
tests, ShellCheck, Rust formatting, Clippy with warnings denied, all test
groups, and the release fail-closed checks. The main Rust suite reported 217
passed and zero failed tests, followed by four passing orchestration tests.

The Linux-only adapter evidence covered:

- exact fixed-property access and schema mapping;
- trusted and wrong owner UID handling;
- present and absent observations;
- timeout and unavailable owner or bus failures;
- malformed percentage and registry metadata rejection;
- stale and future observation rejection; and
- serialization without device identity or history.

The earlier run `34107473574` is diagnostic evidence only: it exposed an
incorrect module path for the D-Bus unique-owner type. Signed commit `8f9b420`
corrected the type import, and the authoritative run above passed unchanged
security semantics.

## Remaining limits

This is deterministic private-bus evidence, not installed-host proof against a
real UPower service. Real battery-present and valid no-battery installed
evidence remains an exit gate for the first Phase 7 slice. The immediate next
checkpoint is the policy, verification, and content-minimized audit route; no
command or executor fallback is permitted.
