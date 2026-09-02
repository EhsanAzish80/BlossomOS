# ADR-0008: Exact systemd service-status reads

- Status: Accepted
- Date: 2026-09-02
- Owners: Project maintainers

## Context

ADR-0004 makes one exact `services.read:status` operation the final Phase 2
capability. Service state is useful system context, but names and failure states
can disclose installed software, security products, private workloads, and host
activity. A broad unit listing would also compose with process and file reads
into a substantially stronger host-inventory interface.

The operation must not recreate command execution through `systemctl`, shell
out, load an otherwise-unloaded unit as a side effect, expose all D-Bus
properties, or let a caller choose a bus destination, object path, interface,
method, property, timeout, or connection address.

systemd documents its manager D-Bus interfaces as stable. `GetUnit` returns the
object path for an already-loaded exact unit and fails when that name is not
loaded; unlike `LoadUnit`, it does not request that systemd load a unit from
disk. Generic unit objects expose the high-level `LoadState`, `ActiveState`, and
the more detailed unit-type-specific `SubState`.

## Decision

### Authority and scope

Add one dedicated request for `services.read:status`, scoped to one exact
system-manager `.service` unit name. The initial accepted name grammar is a
conservative ASCII subset: 1 through 256 bytes, ending in `.service`, with a
non-empty stem containing only ASCII letters, digits, colon, underscore, dot,
at sign, and hyphen. Slash, backslash, whitespace, control characters, glob
metacharacters, empty path-like components, other unit types, D-Bus object
paths, and caller-provided escapes are rejected.

This deliberately excludes some valid systemd escaped names. Supporting a
broader grammar requires a later compatibility and scope review; the request
must never invoke systemd's name-mangling rules on ambiguous input.

The policy decision is `ask` for every exact unit. The interactive preview must
show the exact unit, system-bus destination, fixed read-only calls, bounded
result fields, and the absence of command, mutation, privilege, network, and
generic D-Bus authority. Only `Approve once` reaches the bus. Denial,
cancellation, expiry, replay, and non-interactive use make no D-Bus connection
or call.

### Code-owned D-Bus operation

The Linux implementation connects only to the local system bus using the
library's standard system-bus transport. It communicates only with:

- destination `org.freedesktop.systemd1`;
- manager path `/org/freedesktop/systemd1`;
- manager interface `org.freedesktop.systemd1.Manager`;
- method `GetUnit`, with the approved exact unit as its sole argument; and
- the returned object path, using only read-only property `Get` calls on
  `org.freedesktop.systemd1.Unit` for `Id`, `LoadState`, `ActiveState`, and
  `SubState`.

It must use `GetUnit`, never `LoadUnit`, `ListUnits`, `GetAll`, subscription,
signals, a type-specific service interface, or any method that changes manager
or unit state. It must not invoke `systemctl`, another process, the Phase 1
executor, or a shell. Callers cannot supply connection addresses, destinations,
paths, interfaces, methods, properties, credentials, or timeouts.

The provider uses a fixed short operation deadline and bounded D-Bus message
limits where the selected library supports them. Connection, lookup, property,
protocol, timeout, unavailable-unit, unavailable-systemd, and over-bound result
failures are distinct typed non-successes. There is no command fallback.

### Result and verification

The result contains only:

- the requested exact unit name;
- system scope;
- systemd's returned canonical `Id`;
- `LoadState`;
- `ActiveState`;
- `SubState`; and
- fixed provenance identifying the systemd D-Bus destination and interfaces.

Every string has a small fixed UTF-8 byte bound and rejects NUL/control
characters. The canonical `Id` must itself be a valid bounded `.service` name.
States are opaque bounded tokens rather than a closed enum because systemd may
add state values compatibly. The operation does not expose description,
fragment paths, documentation, dependencies, process IDs, cgroups, timestamps,
exit status, status text, environment, logs, job data, or unit-file enablement.

Property reads are observations, not an atomic snapshot: a service may
legitimately transition while the calls are in flight. Verification therefore
binds the approved requested unit and fixed provenance, validates the narrow
schema, and confirms that no unapproved field entered the result. It does not
claim the state remains current after the response.

### Audit and privacy

The exact requested and canonical unit names are sensitive and are not stored
in clear text in audit records. Audit stores domain-separated digests of those
names, the system scope, fixed provider/provenance identifiers, transition and
error class, verification outcome, and hash-chain identifiers. Load, active,
and sub-state values are rendered to the user but omitted from persistent audit
details.

The activity view shows the approved unit and returned status for the current
interactive result, plus request, decision, verification, and audit IDs. It
must not imply that a read was a durable snapshot or that the service is safe,
healthy, enabled, or correctly configured.

### Required evidence

Before the implementation merges, tests must cover:

- strict request/result schemas, unknown fields, size limits, unit-name grammar,
  aliases, templates, malformed UTF-8/control data, and unexpected state tokens;
- exact approval binding, denial, cancellation, expiry, replay, and non-TTY
  zero-call behavior;
- fixed destination/path/interface/method/property ownership and proof that
  `LoadUnit`, `ListUnits`, `GetAll`, mutating methods, subprocesses, and the
  generic executor are never used;
- unavailable bus, unavailable systemd, unknown/unloaded units, access denial,
  malformed replies, timeout, disconnect, and state transitions between
  property reads;
- result verification, current-result escaping, audit redaction, and
  cross-tool composition review; and
- target-Linux integration against a controlled mock D-Bus service plus a
  non-mutating real-systemd smoke test when the CI environment actually runs a
  usable system manager. Environment absence must be reported as unavailable,
  never converted into false success.

## Alternatives considered

- `systemctl show` or `is-active`: rejected because it launches a process and
  relies on a command-backed boundary when a stable native API exists.
- Reading unit files, cgroups, or `/proc/1`: rejected because these do not
  authoritatively represent the manager's current service state.
- `LoadUnit`: rejected because a read request must not cause systemd to load an
  otherwise-unloaded unit.
- `ListUnits` or `ListUnitFiles` followed by filtering: rejected because it
  unnecessarily retrieves a host-wide inventory.
- `GetAll` on the exact unit: rejected because it retrieves sensitive fields
  outside the public result schema.
- Default allow because the operation is read-only: rejected because exact
  service presence and state are private host metadata.
- A generic D-Bus tool: rejected because destination, interface, method, and
  arguments collectively form broad IPC authority.

## Security and privacy consequences

The capability can reveal whether one user-named loaded service exists and its
coarse current state, but only after exact once-only approval. It cannot list
services, load units, inspect processes or logs, mutate systemd, or send an
arbitrary D-Bus message. Combined with earlier Phase 2 tools, it still cannot
express generic execution or ambient host inventory without a human selecting
and approving each exact service.

The local system bus and systemd are additional trusted dependencies. D-Bus
policy may deny reads, a service can transition during observation, and a
compromised manager can return misleading data. Blossom reports those limits
and does not elevate privileges or bypass bus policy.

## Operational consequences

The implementation may add a Rust D-Bus client dependency after dependency and
license review. Test doubles must implement only the fixed surface above. The
Linux provider is unavailable on unsupported platforms, and no compatibility
fallback may invoke a command.

## Migration and rollback

The capability is independently removable by deleting its request/result,
policy mapping, provider, verifier, renderer, audit fields, and tests. It adds
no durable grant or persistent service data.

Any expansion to user-manager units, other unit types, service listing,
additional properties, subscriptions, service logs, mutations, or a broader
name grammar requires a new ADR that reviews privacy and composition.

## Validation

- Review against ADR-0004 and the existing policy, approval, verification, and
  audit contracts.
- Review against systemd's stable D-Bus manager and generic Unit interfaces.
- Protected CI must pass before implementation or roadmap completion claims.
