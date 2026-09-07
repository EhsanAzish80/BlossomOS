# Phase 7 installed battery evidence

Status: complete for the first fixed battery-summary slice on 2026-09-07.

The authoritative evidence is GitHub Actions run
[`34117027739`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34117027739),
which passed against signed commit
`225ac295d81f86e3839a86f86994fdd51948c9b5`.

## Real battery-present boundary

Job
[`101726143732`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34117027739/job/101726143732)
ran on the dedicated `blossom-x64-phase6` x86-64 Ubuntu host
`ehsan-MacBookPro11-1`. The gate required the real system-bus socket, an active
UPower service, and Docker before checkout. A digest-pinned GNU Rust container
built the production probe from the exact checked-out source; the resulting
binary then ran directly on the host, outside the container, against the host's
real root-owned UPower service.

The fixed adapter returned:

```text
battery-evidence=present state=full schema=1 source=system.battery.summary
```

The evidence output intentionally omits percentage, device path, vendor,
serial number, and other hardware identity.

## Real no-battery boundary

Job
[`101726144116`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34117027739/job/101726144116)
ran on an x86-64 Ubuntu hosted runner. It installed and started real UPower,
required that neither `BAT0` nor `BAT1` existed, and ran the same production
adapter from the same commit.

The fixed adapter returned:

```text
battery-evidence=absent schema=1 source=system.battery.summary
```

Aggregate job
[`101726905835`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34117027739/job/101726905835)
passed only after both real outcomes passed.

## Independent first-slice exit audit

- The source is a closed enum member, not caller-selected discovery.
- The singleton capability remains policy-routed and default-deny.
- The adapter uses one fixed destination, display-device path, interface, and
  property allowlist with owner checks and a code-owned deadline.
- Present, absent, unavailable, untrusted-owner, property, timeout, and protocol
  failures remain distinct and fail closed.
- Verification bounds schema, freshness, lifetime, percentage, and response
  size before projection.
- Audit and installed evidence omit battery percentage and device identity.
- No command, sysfs, generic D-Bus, or QML authority fallback was added.
- Both required real installed outcomes passed on the same signed source.

Conclusion: checkpoint 6 and the first fixed battery-summary slice satisfy the
ADR-0022 exit boundary. This does not close broader Phase 7.

## Regression evidence

- Quality run `34114055508` passed at checkpoint-5 evidence commit `81308da`.
- Installed shell regression run `34112782428` passed at implementation commit
  `3186adc` on the same trusted Intel runner.
- The battery-only dispatch deliberately skipped the compositor job. It tested
  only the new installed battery boundary and did not replace the earlier
  authoritative Phase 6 installed-shell run.

## Evidence limits

This proves the fixed Linux GNU UPower adapter distinguishes a real present
battery from a real absent battery while preserving the closed schema and
content-minimized output. It does not prove broad laptop compatibility,
battery accuracy, hardware enumeration, arbitrary system D-Bus access,
packaging, installer, distribution image, upgrade, rollback, or release
readiness. Later Phase 7 sources remain separate reviews and checkpoints.
