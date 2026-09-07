# ADR-0022: Typed context registry and battery summary

- Status: Accepted
- Date: 2026-09-07
- Accepted: 2026-09-07 after project-owner review of the Phase 7 first slice
- Owners: Project maintainers

## Context

Phase 7 introduces system and desktop observations that the shell and later the
agent runtime may use as context. These observations are not harmless merely
because they are read-only. Window titles, notifications, clipboard content,
network identifiers, filenames, and device identifiers can expose private
activity or create a broad surveillance interface when combined.

A generic D-Bus proxy, filesystem reader, property browser, screenshot stream,
or caller-selected polling API would bypass the closed capability model built
in earlier phases. Context also becomes misleading when its source, freshness,
absence, or failure is hidden. The first Phase 7 slice must therefore establish
the registry and observation rules with one low-content source before private
desktop sources are considered.

## Decision

### Closed context registry

Every context source is a code-owned Rust type registered at build time. A
source owns its stable identifier, schema version, required capability, privacy
class, maximum response size, freshness lifetime, rate limit, native adapter,
result validator, redaction rules, and tests. Callers cannot register sources or
supply a capability, path, command, D-Bus destination, object, interface,
property, polling interval, retention period, or verifier.

The first registered source is `system.battery.summary`, protected by the new
singleton capability `system.read:battery.summary`. Default deny remains
universal. Tests may install an explicit allow rule; no production policy is
silently enabled by this ADR.

Every successful observation is returned in a size-bounded envelope containing
only:

- the fixed source identifier and schema version;
- a service-authored observation timestamp and maximum age;
- one of `present` or `absent`; and
- the validated typed value when present.

Source failure, malformed data, timeout, authorization failure, and stale data
are explicit errors rather than `absent`, zero, or cached success. The first
slice performs a fresh bounded read and adds no persistence. A consumer must not
present an observation after its maximum age and cannot repair or extend it.

### Fixed battery schema

The battery value contains only:

- charge percentage as an integer from 0 through 100; and
- state as `charging`, `discharging`, `full`, `not_charging`, or `unknown`.

It excludes manufacturer, model, serial number, native device path, chemistry,
capacity history, energy rate, voltage, temperature, charge-cycle history,
timestamps of user activity, and time-to-empty/full estimates. Multiple
physical batteries are represented only by the system display aggregate; no
device list or stable fingerprint is exposed.

`absent` is a valid verified observation only when the fixed native source
reports that no display battery is present. An unavailable service or malformed
reply is never converted to absence.

### Native Linux boundary

The initial Linux adapter uses the system D-Bus and the fixed UPower display
device. It may contact only:

- destination `org.freedesktop.UPower`;
- object `/org/freedesktop/UPower/devices/DisplayDevice`;
- interface `org.freedesktop.UPower.Device`; and
- individually named `Type`, `State`, `Percentage`, and `IsPresent` properties
  through `org.freedesktop.DBus.Properties.Get`.

`GetAll`, enumeration, introspection, signals, device paths returned by the
service, and caller-selected destinations or properties are prohibited. The
complete read has a two-second deadline and a 4 KiB reply limit. Values are
strictly type-checked; non-finite or out-of-range percentages fail closed.
Before and after the property reads, the adapter resolves the fixed well-known
name to one unique owner and requires the system bus to report UID 0 for that
owner. Missing, changing, or non-root ownership invalidates the observation.

No command, shell, subprocess, filesystem scan, network request, privileged
helper, screenshot, or accessibility automation is used. The adapter is an
unprivileged client of an existing system service and has no mutation method.

### Consumers and authority

An observation conveys data, never permission. It cannot approve a request,
select a tool, expand a scope, change policy, or prove user intent. Model access
to context is not introduced by this slice and requires a separately reviewed,
minimal projection.

QML remains an untrusted presentation layer. If battery status is later shown
in the shell, a narrow native client may expose only the fixed typed projection;
QML does not receive generic D-Bus or context-registry access.

The audit records request identity, source identifier, policy result, adapter
outcome, verification outcome, and error category. It does not record charge
percentage or reconstruct a battery history.

## Alternatives considered

### Read `/sys/class/power_supply` directly

Deferred. The kernel interface is native and avoids a service dependency, but
class entries commonly cross symlinks into device-specific trees and require a
separately reviewed discovery, containment, hotplug, and multi-battery
aggregation design.

### Expose all UPower devices or properties

Rejected. Enumeration, `GetAll`, stable device paths, and hardware metadata
would expand a low-content singleton into a fingerprinting and discovery API.

### Let the shell call UPower directly

Rejected. That would bypass Blossom's typed capability, freshness, validation,
audit, and future per-source permission boundary.

### Build the full Phase 7 registry in one change

Rejected. Clipboard, notifications, selected files, windows, applications, and
projects have materially different privacy and lifetime rules. Each must be a
separate reviewed increment after the first source proves the common boundary.

## Security and privacy consequences

The fixed aggregate reveals coarse charge state already available to local
desktop software while withholding device identity and historical behavior. A
compromised same-user caller can request the source but still receives data only
after the code-owned policy decision. Rate limiting and bounded replies prevent
the context service from becoming an uncontrolled polling or allocation path.

UPower and the system bus are trusted observation dependencies, not authority
providers. Spoofed peers, wrong owners or UIDs, wrong types, service replacement,
disconnect, timeout, malformed values, and stale observations fail closed.
Battery data must never be treated as proof that a particular person or device
is present.

This ADR creates no durable context or history. Retention, personalization, and
memory remain Phase 8 concerns.

## Operational consequences

The production adapter requires a reviewed UPower package/runtime dependency.
The core registry and validator remain testable with deterministic fixtures.
Linux transport tests use a private bus and fake fixed service; installed
evidence must additionally exercise the real system UPower service on a machine
with a battery and the valid `absent` result on a machine without one.

Unsupported hosts and missing UPower report `unavailable`; they do not fall back
to commands, sysfs scanning, fabricated values, or a broader API. The feature
remains inactive until implementation and installed evidence pass.

## Migration and rollback

The registry, capability, request, adapter, and optional shell projection are
additive. Removing the Phase 7 package or disabling its inactive service returns
to the completed Phase 6 surface without altering existing capabilities,
approval, execution, verification, or audit formats.

Changing the source schema, native provider, property set, privacy class,
freshness, policy, or consumer projection requires explicit review. Accepted
ADRs are superseded rather than silently redefined.

## Validation

Before the slice may be marked complete, tests and evidence must prove:

- the source, capability, D-Bus destination, object, interface, and four
  property names are fixed in code;
- default deny and explicit allow behavior use the existing policy boundary;
- present and absent observations are distinct from every failure category;
- wrong owners, types, property names, enum values, percentages, oversized
  replies, timeouts, disconnects, and service replacement fail closed;
- no identifiers, paths, history, percentage, or private value enter audit;
- responses and polling are bounded and stale results cannot be presented as
  current;
- QML and model output cannot select or widen a source;
- existing Phase 1-6 tests and protected repository checks remain green; and
- target-Linux installed evidence exercises real present and absent behavior
  while recording its hardware and distribution limitations.
