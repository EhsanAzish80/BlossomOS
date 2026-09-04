# ADR-0021: Shell IPC and approval surfaces

- Status: Proposed
- Date: 2026-09-04
- Owners: Project maintainers

## Context

Phase 6 introduces Blossom's first graphical client: a Quickshell/QML shell on
Hyprland. It will display requests, policy decisions, exact approval previews,
verified outcomes, and audit activity. It must not become a second broker or
gain ambient authority merely because it is a trusted-looking desktop surface.

The shell is a same-user process. It can be compromised, imitated, restarted,
or driven with stale state. QML text and model-generated content are untrusted.
Approval can therefore mislead unless the service binds what is displayed, what
is approved, who requested it, and what is executed to one code-owned identity.

Phase 6 also needs an IPC boundary. A generic RPC, command, or object-discovery
surface would expose more authority than the existing closed Rust types. The
first slice must prove UI integration without expanding the tool catalogue,
filesystem scopes, privilege surface, planning language, or model authority.

## Decision

### Trust boundary

The Quickshell process is an untrusted presentation client. It may request a
registered operation, render service-authored projections, submit a one-time
human decision, cancel work that has not started, and read redacted activity.
It cannot create policy decisions, capabilities, scopes, approval tokens,
execution results, verification results, or authoritative audit events.

Hyprland is the supported compositor target, but focus, window identity,
layer-shell placement, and visual prominence are usability signals rather than
authorization evidence. The broker remains authoritative.

### Session IPC

The first transport is the authenticated per-user D-Bus session bus. A
code-owned Blossom service owns a fixed name, object path, interface, and
version. It accepts only closed, size-bounded methods and signals projected from
existing typed requests and orchestration states.

The service derives the caller's unique bus name and credentials from the bus.
Payloads cannot supply or override sender identity, UID, PID, capability,
resource scope, policy result, approval lifetime, or audit identity. Unknown
interfaces, methods, fields, variants, schema versions, oversized values,
duplicate decisions, stale request IDs, and disconnected callers fail closed.

D-Bus authentication establishes a local peer, not user intent or trust in the
shell. Any same-session process may attempt a call. Every operation therefore
still enters the existing typed policy, approval, execution, verification, and
audit path. The session service has no direct privileged-helper authority.

The initial interface is intentionally small:

- request the existing fixed `uname` diagnostic;
- subscribe to a bounded, redacted activity projection;
- obtain a service-authored exact preview for one pending request;
- submit `approve_once` or `deny` for that request; and
- request cancellation before execution has started.

No generic command, arguments, path, D-Bus destination, tool discovery, model
prompt, arbitrary plan, audit query, or privileged operation is accepted.

### Approval ceremony

The service creates a fresh opaque request ID and immutable approval challenge.
Its structured preview includes the operation and purpose, exact executable and
fixed arguments where applicable, capability and scope, filesystem/network/
privilege/side-effect summary, request correlation and expiry, and an opaque
digest binding the complete normalized preview.

The shell renders untrusted explanations separately from fixed security fields.
It never uses model-generated labels for an operation, scope, button, or result.
`Approve once` and `Deny` are the only Phase 6 choices. There is no permanent,
session-wide, wildcard, or bulk approval.

Approval submission contains only request ID, challenge digest, and decision.
The service checks that the challenge is pending, unexpired, unchanged, bound to
the submitting D-Bus connection, and unused, then creates the existing private
one-use grant. The approval token never crosses IPC and is never rendered,
logged, or stored by QML. Mutation, replay, timeout, shell/service restart,
connection loss, or duplicate submission denies and starts nothing.

Only one approval may be visibly pending per shell connection. A new request
cannot replace or cover it. Keyboard shortcuts cannot approve; approval needs a
deliberate pointer or touch activation after all security fields are visible.
Focus loss does not approve or dismiss. Escape and window close deny.

### Activity and truthful outcomes

The shell reads a bounded projection of existing hash-chained audit records and
shows request, policy, approval, execution, verification, terminal outcome, and
correlation IDs. It does not treat a D-Bus reply, exit status, provider text,
notification, animation, or optimistic state as success.

Only code-owned verified outcomes render as completed. Missing events, sequence
gaps, service loss, parsing failure, and indeterminate results remain explicit.
The UI never reconstructs or repairs authoritative audit history.

### Process and packaging boundary

Quickshell is a separate unprivileged process. QML does not load the Rust core
as a library or spawn the CLI, Bubblewrap, provider, helper, `systemctl`, a
shell, or arbitrary executables. The service owns no graphical surface and
accepts no QML or JavaScript program as data.

Phase 6 pins supported Hyprland and Quickshell interfaces before compatibility
claims. Other hosts may use protocol fixtures, but exit evidence must use pinned
Arch userspace with real Hyprland, Quickshell, session D-Bus, and packaged
Blossom services. This is not hardware, installer, ArchISO, or release proof.

### First vertical slice and non-goals

```text
shell request -> typed service request -> policy -> exact preview
              -> approve once or deny -> fixed sandboxed uname
              -> verification -> redacted activity projection
```

The slice adds no capability. Agent chat, free-form prompts, arbitrary plans,
file portals, window control, launcher actions, notifications, system status,
durable approval, raw audit content, generic Bash, sudo, and new privileged
operations remain out of scope until separately reviewed.

## Alternatives considered

### Let QML call the CLI or Rust core directly

Rejected. It collapses presentation, authorization, token custody, and
execution into one compromise domain and makes caller identity ambiguous.

### Use a private Unix socket with a custom protocol

Deferred. It can provide strong framing but duplicates session discovery,
credential handling, activation, and schema tooling. D-Bus is adequate only
because peer identity is routing evidence rather than authorization.

### Trust only the official shell client

Rejected. Same-user clients can imitate or compromise a graphical process. The
service must remain safe when called by a hostile session peer.

### Add launcher, chat, and approvals together

Rejected. A larger first surface obscures whether approval and truthful
activity preserve the existing boundary.

## Security and privacy consequences

The design keeps approval tokens and execution authority outside QML and binds
each decision to an exact immutable preview. It limits confused-deputy, replay,
stale-state, prompt-injection, and visual-substitution risks. It cannot protect
trusted pixels or input after compositor, input stack, kernel, root, firmware,
or physical-machine compromise.

Activity remains privacy-sensitive. Its projection is bounded, local-only, and
content-minimized. The initial surface exposes no prompts, file contents,
credentials, tokens, raw output, or provider reasoning.

## Operational consequences

Phase 6 adds Quickshell, Hyprland, a per-user D-Bus service, versioned interface
definitions, and UI integration tests. Service and shell remain independently
testable with fixed fixtures. Unsupported versions fail visibly rather than
silently weakening authorization.

## Migration and rollback

The shell and service are additive and inactive by default until packaging and
end-to-end evidence pass. Existing CLI and core tests remain reference behavior.
Removing Phase 6 packages returns to non-graphical interfaces without changing
capability, policy, approval, executor, helper, or audit formats.

## Validation

Tests and evidence must prove:

- hostile same-user callers cannot forge identity, authority, approval state,
  tokens, capabilities, scopes, outcomes, or audit records;
- malformed, oversized, stale, replayed, disconnected, reordered, and
  version-mismatched IPC fails closed;
- displayed digest and fixed fields match the exact normalized executed request;
- denial, expiry, cancellation, close, service loss, bus restart, and shell
  restart start nothing when approval has not been consumed;
- approval is one-use, connection-bound, mutation-resistant, and never exposed
  to QML, logs, screenshots, or activity;
- only verified results render completed, with gaps and uncertainty explicit;
- QML cannot spawn commands or select tools, arguments, paths, plans, D-Bus
  targets, or privileged requests;
- focus, overlays, notification spoofing, rapid replacement, and accessibility
  behavior receive adversarial UI tests;
- existing Phase 1-5 negative tests remain green; and
- pinned Arch/Hyprland/Quickshell installed evidence exercises the complete
  fixed diagnostic slice before Phase 6 is marked complete.

