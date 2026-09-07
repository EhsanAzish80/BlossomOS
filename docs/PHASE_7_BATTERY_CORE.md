# Phase 7 battery context core

Status: implemented as the first ADR-0022 checkpoint. No native adapter,
production policy, shell projection, model context, or installed-runtime claim
is included.

## Closed registry

The core registry currently contains exactly one source:
`system.battery.summary`. It derives the singleton
`system.read:battery.summary` capability in code and fixes protocol version 1,
a 5-second maximum age, a 1-second minimum polling interval, and a 4 KiB maximum
serialized response.

The capability remains denied by the default policy. An explicit typed policy
rule can allow it, but no production rule is installed by this checkpoint.

## Typed observation

A battery observation is either `present` with an integer percentage from 0
through 100 and a closed charging-state enum, or `absent` without a value.
Validation rejects the wrong source, unsupported schema, modified lifetime,
future timestamp, stale observation, invalid percentage, and oversized encoded
response.

The projection contains no manufacturer, model, serial number, device path,
history, temperature, voltage, or time estimate. It performs no I/O and creates
no cache or durable history.

## Evidence

On 2026-09-07:

- all 194 `blossom-core` unit tests and four orchestration integration tests
  passed outside the restricted local sandbox;
- the new registry, default-deny/explicit-allow, present/absent, enum,
  percentage, freshness-boundary, fixed-metadata, serialization, and privacy
  tests passed;
- `cargo clippy --locked -p blossom-core --all-targets -- -D warnings` passed;
- `scripts/ci/check_repository.py` and `git diff --check` passed; and
- the repository contained no prohibited legacy distribution reference.

The first sandboxed full-suite attempt could not bind existing loopback test
sockets and failed with `Operation not permitted`. The unchanged suite passed
when rerun with the required local socket permission; this was an environment
restriction, not a product failure.

## Remaining gate

The next checkpoint is the fixed UPower system-D-Bus adapter. Until its owner,
property, timeout, replacement, present/absent, and failure tests pass, this
core schema makes no claim about reading a real battery.
