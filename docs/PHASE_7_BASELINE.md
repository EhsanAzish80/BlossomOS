# Phase 7 structured system awareness baseline

Status: active. ADR-0022 was accepted on 2026-09-07 after project-owner review.
Implementation proceeds through the ordered checkpoints below.

Phase 7 begins from the completed Phase 6 shell and session-service boundary.
Its goal is to add typed, permissioned, observable, and testable context without
creating a generic discovery API or invisible personal history.

## Ordered implementation checkpoints

1. Review and accept ADR-0022 for the closed context registry and fixed battery
   summary. Complete.
2. Add the registry envelope, typed battery result, singleton capability, strict
   validator, and deterministic unit tests. Keep default deny.
3. Add the fixed UPower system-D-Bus adapter with peer checks, exact property
   allowlist, timeout, reply bound, and private-bus adversarial tests.
4. Route the battery request through existing policy, verification, and
   content-minimized audit paths. Add no command or executor fallback.
5. Add a narrow shell projection only after the service boundary passes. Keep
   generic D-Bus and registry discovery out of QML.
6. Produce installed present-battery and no-battery evidence, protected
   regression results, and an independent first-slice audit.
7. Propose later sources one at a time. Private sources such as window titles,
   clipboard, notifications, selected files, and active projects require their
   own privacy and lifetime review.

## First slice

```text
fixed battery request
  -> code-owned capability and policy
  -> fixed UPower display-device properties
  -> strict typed validation
  -> fresh present/absent observation
  -> content-minimized audit outcome
  -> optional narrow shell projection
```

## Non-goals

This baseline adds no hardware enumeration, device identity, arbitrary system
D-Bus, sysfs browsing, commands, screenshots, accessibility scraping, polling
chosen by callers, durable context, model context, window titles, clipboard,
notifications, file access, project discovery, or system mutation.

## Exit evidence for the first slice

- Accepted ADR-0022.
- Closed source and capability types with default deny.
- Fixed native adapter and strict present/absent/error distinction.
- Bounded freshness, response size, polling, and audit data.
- Adversarial D-Bus, stale-state, replacement, and privacy tests.
- Narrow shell presentation with no new QML authority, if included.
- Real installed Linux evidence for a battery-present machine and a valid
  no-battery machine.
- Passing Phase 1-6 regression, lint, dependency, secret, and CodeQL checks.
- Independent evidence document that preserves hardware, packaging,
  distribution-image, and release limitations.

The broader Phase 7 exit remains unchanged: every added context source must be
typed, permissioned, observable, and testable.
