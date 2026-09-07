# ADR-0023: Fixed network connectivity summary

- Status: Accepted
- Date: 2026-09-07
- Accepted: 2026-09-07 for the next bounded Phase 7 source
- Owners: Project maintainers

## Context

Phase 7 needs enough network awareness to report whether the local system is
offline or has usable connectivity. Network information is privacy-sensitive:
SSIDs, BSSIDs, interface names, addresses, routes, DNS servers, access-point
lists, signal strength, traffic counters, and connection history can identify a
person, place, device, or routine. A generic NetworkManager, D-Bus, netlink, or
filesystem interface would also bypass the closed context registry.

## Decision

Add exactly one source, `system.network.connectivity`, protected by the
singleton capability `system.read:network.connectivity`. The request has no
arguments. Production policy remains default-deny.

The version-1 value is one closed state:

- `offline`: NetworkManager reports no connectivity;
- `local`: a local network is available without confirmed Internet access;
- `limited`: connectivity is being established or a portal is suspected;
- `online`: NetworkManager reports full connectivity; or
- `unknown`: NetworkManager returned its documented unknown state.

Absence is not a valid result for this singleton. A missing service, malformed
or undocumented value, timeout, authorization failure, owner replacement, or
unsupported host is an explicit error.

The Linux adapter may contact only the system-bus destination
`org.freedesktop.NetworkManager`, object `/org/freedesktop/NetworkManager`, and
interface `org.freedesktop.NetworkManager`. It reads only the individually
named `Connectivity` property through
`org.freedesktop.DBus.Properties.Get`. It resolves the fixed well-known name
before and after the read, requires one unchanged unique owner, and requires
the system bus to report UID 0 for that owner. The complete read has a
two-second deadline and a 4 KiB response bound.

The adapter must not use `GetAll`, introspection, enumeration, signals, commands,
subprocesses, `/sys`, `/proc/net`, netlink, sockets to remote hosts, DNS probes,
HTTP probes, screenshots, or accessibility automation. Callers cannot select a
destination, object, interface, property, probe target, timeout, polling rate,
or verifier.

Observations use the existing typed envelope with a five-second maximum age and
a one-second minimum poll interval. Verification checks the fixed source,
schema, lifetime, freshness, response bound, and closed state. Audit records
only request identity, source, policy, provider outcome, verification outcome,
and error category; it never records the connectivity value.

QML and the model receive no generic network or registry authority. Any shell
projection must be a separately tested fixed enum. The observation conveys
data, never permission or proof that a remote service is reachable.

## Alternatives considered

### Enumerate interfaces or Wi-Fi access points

Rejected. Names, addresses, radio identifiers, SSIDs, and nearby networks add
fingerprinting and location exposure without being necessary for coarse
connectivity awareness.

### Perform an Internet or DNS probe

Rejected. A probe creates externally visible traffic, leaks timing and target
information, and turns a local read into a network side effect.

### Read NetworkManager `State` or all properties

Rejected for this slice. The single documented connectivity property is enough
for the closed schema. Additional properties would widen the trusted parser and
require their own need and privacy review.

## Security and privacy consequences

The result reveals only a short-lived coarse connectivity class. It cannot
identify the interface, network, device, location, address, route, or traffic.
NetworkManager and the system bus are trusted observation dependencies, not
authority providers. Wrong ownership, service replacement, malformed values,
oversized replies, timeout, disconnect, and stale observations fail closed.

No observation is persisted. Retention and personalization remain Phase 8.

## Validation

Before this source is complete, tests and evidence must prove:

- the source, capability, destination, object, interface, and property are
  fixed in code;
- default deny and explicit allow use the existing policy boundary;
- every documented state maps deterministically and other values fail closed;
- wrong owners, wrong UIDs, replacement, wrong types, oversized replies,
  timeout, and disconnect fail closed;
- values and network identifiers never enter audit;
- responses, freshness, and polling are bounded;
- no external network traffic or generic QML authority is introduced;
- protected repository checks remain green; and
- installed Linux evidence exercises real offline and online-class outcomes,
  while recording the host and service limitations.

## Migration and rollback

The source is additive. Removing it returns to the completed battery-only Phase
7 registry without changing earlier capabilities or audit formats. Any schema,
provider, property, privacy, lifetime, policy, or consumer change requires a
new review rather than silently changing this decision.
