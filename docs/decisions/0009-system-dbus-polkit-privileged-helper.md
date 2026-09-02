# ADR-0009: System D-Bus and polkit for the privileged helper boundary

- Status: Accepted
- Date: 2026-09-02
- Owners: Project maintainers

## Context

Phases 1 and 2 prove that unprivileged operations can remain typed,
deny-by-default, approval-bound, verified, and audited without exposing a shell.
Phase 3 must eventually perform one narrow operation that genuinely needs
elevated authority. Running an approved command through `sudo`, `pkexec`, a
setuid wrapper, or a caller-configurable root service would collapse the
capability model into generic root execution.

The privileged boundary must assume that model output, tool output, request
payloads, the graphical shell, and any same-user client are untrusted. It must
authenticate the actual caller independently, bind authorization to one exact
typed operation, reject replay and ambiguity, and fail closed if authorization,
audit, or verification is unavailable.

Polkit is designed for a privileged mechanism to authorize requests from an
untrusted subject. The system D-Bus already authenticates local connections and
lets polkit identify a subject by its unique system-bus name. This avoids a
caller-supplied PID and its reuse race. D-Bus also provides a closed schema,
bounded messages, service policy, activation, and disconnect identity suitable
for an independently packaged helper.

## Decision

### Service and transport

The future helper is a small Rust system service with fixed identifiers:

- well-known name: `org.blossomos.Privileged1`;
- object path: `/org/blossomos/Privileged1`;
- interface: `org.blossomos.Privileged1`; and
- system bus only at the distribution-owned local Unix socket.

It is activated and supervised by systemd. It never listens on TCP, a custom
network socket, the session bus, or a caller-selected address. The D-Bus service
file, bus policy, systemd unit, polkit action declarations, binary path, and
sandboxing directives are code-owned packaging artifacts reviewed with the
first operation.

Any implementation must use the authenticated unique sender from the received
D-Bus message. A request field may not supply or override UID, PID, bus name,
session, executable identity, authorization subject, or polkit action.

### Closed operation registry

There is no generic `Execute`, `Run`, `Shell`, `Call`, `WriteFile`, `SetProperty`,
or arbitrary D-Bus proxy method. Every privileged operation receives:

- its own method and closed versioned request/result types;
- one statically derived Blossom capability and one fixed polkit action ID;
- exact argument and result bounds with unknown-field rejection;
- a code-owned native implementation and resource profile;
- deterministic post-operation verification;
- redacted broker and helper audit transitions; and
- operation-specific target-Linux negative and adversarial tests.

The initial interface exports no operation until a later ADR selects one. Adding
or changing a method, action, writable path, Linux capability, systemd unit
permission, or polkit default is a security-boundary change requiring focused
review. A privileged operation may not accept an executable, argument vector,
environment, working directory, mount, namespace, unit name, filesystem path,
bus destination, interface, method, property, or timeout unless its own ADR
declares and bounds that exact field.

### Authorization sequence

For each operation the helper performs this sequence:

1. Receive one bounded, schema-valid request and capture the unique D-Bus sender.
2. Resolve the sender's authenticated system-bus credentials and require a
   local non-root user in an active local session. Missing or inconsistent
   credentials deny the request.
3. Derive the fixed polkit action from the method, never from payload data.
4. Ask `org.freedesktop.PolicyKit1.Authority.CheckAuthorization` about the
   `system-bus-name` subject for that captured unique sender.
5. Permit user interaction only for a live explicit request. A non-interactive
   call uses no interaction flag and treats a challenge as denial.
6. Require `is_authorized = true`; a challenge, dismissal, timeout, authority
   restart, disconnect, malformed result, or unavailable authentication agent
   is denial.
7. Revalidate that the same unique sender still owns the request connection and
   that the request digest is unchanged.
8. Atomically claim its idempotency key, record the authorized-start transition,
   perform the one fixed operation, verify the observed result, and record the
   terminal transition.

Polkit is the independent privilege-boundary authorization. Blossom's upstream
preview and once-only approval remain required user-facing policy steps, but no
opaque Blossom approval token is treated as root authority. The helper neither
accepts nor prints approval tokens.

### Polkit policy

Each operation has a dedicated action in a vendor policy file under
`/usr/share/polkit-1/actions`. Initial defaults are:

- `allow_any`: `no`;
- `allow_inactive`: `no`; and
- `allow_active`: `auth_admin`.

Retained variants such as `auth_admin_keep` and `auth_self_keep` are prohibited
for the first operation. The package does not install a JavaScript `.rules`
file: upstream polkit documentation reserves authorization rules for system
administrators and special-purpose OS policy, not application mechanisms.
Administrators may impose stricter local policy, but Blossom does not silently
weaken the declared defaults.

The polkit interaction must display the fixed action description plus bounded,
root-supplied details identifying the exact normalized operation and scope. User
or model text cannot replace the security-relevant action description.

### Freshness, replay, cancellation, and concurrency

Every request carries a broker-generated 128-bit random idempotency key and a
bounded correlation ID. The helper keeps a root-owned, boot-scoped idempotency
journal in its systemd `RuntimeDirectory`, keyed by authenticated UID, operation,
idempotency key, and normalized request digest. It writes and syncs the claim
before mutation.

A duplicate with the same digest returns the already recorded result and never
executes twice. Reuse with a different digest is rejected and audited. The
journal survives helper restarts within the boot, has a fixed entry and byte
bound, and fails closed when it cannot safely record a claim. Entries may expire
only under a documented retention rule after the original polkit authorization
can no longer be reused; reboot clears the runtime journal and any later request
requires a new authorization.

Only one interactive authorization may be outstanding per authenticated UID,
with a small global bound and rate limit. Caller disconnect, user cancellation,
polkit dismissal, or deadline expiry cancels authorization and starts no
operation. Once a non-rollbackable mutation starts, disconnect does not create
an automatic retry or false cancellation; completion and verification continue
to a truthful terminal result.

### Process hardening

The helper runs as root only when the selected operation requires it. Its
systemd service must use the strictest directives compatible with that exact
operation, including a private temporary directory, private devices, a read-only
system view except explicit writable paths, no network address families beyond
local Unix IPC, no new privileges, a fixed umask, memory-execution and personality
restrictions, bounded tasks/memory/runtime, a minimal syscall set, and an exact
capability bounding set. Empty Linux capability sets are preferred when ordinary
root file or D-Bus authority is sufficient.

The helper never spawns a shell or general command. A subprocess is prohibited
for the first operation unless its operation ADR proves that no native API is
viable and fixes the absolute executable, full argument vector, environment,
filesystem, and resource profile.

### Audit and verification

The unprivileged broker and privileged helper both record the same correlation
ID, operation, normalized-scope digest, authenticated UID, polkit action and
outcome, idempotency state, execution state, verification state, and error
category. They do not log credentials, authentication responses, tokens, file
contents, arbitrary user text, or unnecessary personal values.

The helper emits a root-owned transition before and after authorization and
before and after mutation. If the helper audit journal or idempotency journal is
unavailable, the operation does not begin. Success requires an operation-specific
observation independent from the mutation call; issuing a request is never proof
of success.

## Threat analysis

### Malicious model or compromised user-session component

It can construct requests and trigger a polkit challenge but cannot choose an
action, privileged method, root command, or resource outside a compiled method.
It cannot silently pass the fixed `auth_admin` challenge. Prompt spam remains a
denial-of-service risk, reduced by per-UID serialization, rate limits, bounded
queues, exact action text, and audit records.

### Same-user process impersonation

The helper does not trust process names, paths, environment variables, or
caller-supplied credentials. System D-Bus and polkit bind the decision to the
unique bus sender. A same-user process can request authorization but receives no
ambient privilege and cannot reuse another connection's sender identity.

### PID reuse and disconnect races

Polkit receives a `system-bus-name` subject instead of caller-provided PID data.
The helper checks connection ownership after authorization and cancels if the
sender disappears before mutation. The bus name, normalized digest, and
idempotency claim remain bound through the operation.

### Replay and retry ambiguity

The root-owned boot journal makes execution idempotent across client retry and
helper restart. Digest mismatch rejects key substitution. Failures after a
possible side effect return an explicit indeterminate or verification-failed
state; the broker never retries a privileged mutation automatically.

### Policy or authority failure

Missing polkit, missing action metadata, missing authentication agent, malformed
credentials, D-Bus restart, timeout, helper audit failure, or journal failure
denies or returns a truthful non-success result. There is no direct-root fallback.

### Root, kernel, and packaging compromise

These remain outside Blossom's threat guarantee. A malicious root account can
replace the helper or policy. Signed packages, measured release evidence, and
distribution hardening remain later work.

## Alternatives considered

- `sudo` or `pkexec` around an executable: rejected because it authorizes process
  execution and encourages argument/environment authority rather than one typed
  mechanism operation.
- A setuid command: rejected because parsing, loader, environment, and invocation
  behavior become a permanently exposed root attack surface.
- A custom Unix socket: rejected for the first boundary because it would recreate
  peer authentication, activation, policy, cancellation, and message framing
  already supplied by the local system bus.
- Caller PID plus start time: rejected when the authenticated system-bus-name
  subject is available and avoids caller-controlled identity and PID reuse.
- Trust the existing in-process approval token: rejected because the root helper
  has no independent basis for trusting an unprivileged process's opaque token.
- `auth_admin_keep` or permanent approval: rejected because the first privileged
  operation must require fresh explicit authorization without reusable grants.
- Ship a permissive polkit `.rules` file: rejected because it can silently
  override site policy and upstream explicitly limits application use of rules.
- Add the helper and first operation in this ADR: rejected so transport,
  authorization, hardening, and operation-specific side effects receive separate
  protected review checkpoints.

## Security and privacy consequences

This boundary adds a root service and therefore a new high-value attack surface.
Its finite D-Bus interface, independent polkit decision, non-retained default,
boot-scoped idempotency journal, double audit, and systemd restrictions make the
surface reviewable but do not constitute a security guarantee. The project
remains pre-alpha.

System-bus metadata reveals that a UID requested a fixed operation. Operation
ADRs must separately minimize exact arguments and results and must assess what
the polkit prompt, system journal, helper journal, and Blossom activity view
expose.

## Implementation gate

Before the first privileged method is merged, a separate ADR must define:

- the user value and why unprivileged native facilities are insufficient;
- exact request/result schemas, capability, polkit action, and policy preview;
- exact root resources, systemd restrictions, timeout, cancellation, rollback,
  idempotency, and verification semantics;
- bus policy, service activation, packaging ownership and permissions;
- helper/broker audit redaction and correlation;
- a dependency and unsafe-code review; and
- target-Linux tests using isolated fixtures, a controlled bus and polkit
  authority, negative calls from unauthorized subjects, replay/restart cases,
  mutation/verification failures, and proof that no generic root path exists.

No privileged helper code, polkit policy file, D-Bus activation file, systemd
unit, or root operation is introduced by ADR-0009 itself.

## Migration and rollback

There is no released helper or IPC format to migrate. This ADR can be rolled
back as documentation before implementation. Once an interface is implemented,
removing an operation deletes its method, polkit action, policy mapping,
packaging permissions, tests, and audit renderer together; unknown methods and
versions remain rejected.

Changing the transport, subject identity, default authorization, idempotency
model, or generic-operation prohibition requires an ADR that supersedes this
one.

## Validation and references

- Contract review against `ARCHITECTURE.md`, `SECURITY_MODEL.md`, ADR-0004, the
  Phase 2 completion baseline, and the Phase 3 roadmap gate.
- The [polkit overview and action policy specification](https://polkit.pages.freedesktop.org/polkit/polkit.8.html)
  define the mechanism/subject model, action files, defaults, and application
  restrictions on authorization rules.
- The [polkit Authority interface](https://polkit.pages.freedesktop.org/polkit/eggdbus-interface-org.freedesktop.PolicyKit1.Authority.html)
  defines `CheckAuthorization`, `system-bus-name` subjects, interaction flags,
  and authorization results.
- The [D-Bus specification](https://dbus.freedesktop.org/doc/dbus-specification.html)
  defines authenticated connection credentials and system service activation.
- The [dbus-daemon manual](https://dbus.freedesktop.org/doc/dbus-daemon.1.html)
  defines packaged system-service policy and systemd activation integration.
