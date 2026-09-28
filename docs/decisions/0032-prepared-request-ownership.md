# ADR-0032: Prepared request ownership across approval

- Status: Accepted
- Date: 2026-09-28
- Accepted: 2026-09-28 after project-owner review
- Owners: Project maintainers
- Requires: ADR-0005, ADR-0007, ADR-0021, ADR-0031

## Context

Selection-bearing request JSON currently accepts filesystem identities that
trusted code is meant to derive. Correct resolution opens and retains the
selected object, but a domain-only `ToolRequest` cannot own those descriptors.
Re-resolving after approval would reopen attacker-controlled paths and recreate
the race that retained descriptors prevent.

## Decision

### Wire, domain and prepared layers

Only `ToolRequestWire` deserializes untrusted input. It contains only fields a
user or model actually chooses. Device, inode, timestamps, root identity,
parent identity, digest, mode and configured workspace root never appear on the
wire.

`ToolRequest` is the non-deserializable domain value used for policy, preview,
approval binding, verification and content-free audit. Fixed internal requests
have one named constructor per intent; there is no general public constructor.

`RequestResolver` consumes a wire request and `SessionContext`, opens and
validates selected resources, and returns `PreparedToolRequest`. The prepared
value privately owns both the domain request and any retained provider or
descriptor. It is not `Clone`, `Serialize` or `Deserialize`; only the resolver
constructs it.

### Approval ownership

Beginning approval moves the prepared request into the approval store and
returns a token plus exact preview identity. A decision supplies only the token,
decision and preview digest. Approval moves the same stored prepared request
into execution. Neither client nor engine resubmits, reconstructs, re-resolves
or reopens a request after preview.

Deny, cancel, expiry sweep and peer disconnect remove and drop the prepared
request. The approval lifetime remains at most thirty seconds. Pending prepared
requests are capped per peer and globally. Admission reserves capacity before
resolution opens any resource; failure to reserve therefore opens no descriptor.
The reservation is either filled by the resolved prepared request or released
on every resolution failure. Capacity exhaustion never evicts an existing
request.

### Execution revalidation

Approval binds identity derived with `fstat` from the retained descriptor.
Execution performs `fstat` on that same descriptor and compares every retained
identity field before reading or publishing. Size or ctime changes, including
changes through a hard link, fail closed.

### Origin and audit

Resolution assigns a closed origin: `user_cli`, `model_proposed`, or a named
internal fixed intent. CLI and model workspace creation use the same resolver;
origin rules may narrow accepted names but cannot select another construction
path. Audit records the origin and bounded digests/lengths, never file content
or clear workspace paths.

## Alternatives considered

### Re-resolve after approval

Rejected. It approves one observed identity and executes a newly opened path.

### Store only a cloned domain request

Rejected. It loses retained descriptors and permits multiple apparent owners of
the authority granted by one approval.

### Keep selection identities on the wire

Rejected. Validation of caller-supplied identity is not derivation and cannot
establish containment.

## Security and privacy consequences

The approval store temporarily owns live descriptors and is therefore bounded
by lifetime, per-peer count and global count. All terminal paths must remove the
entry so Rust drop closes descriptors. The preview digest rather than a
client-supplied request binds the decision.

A hostile process running as the desktop user can occupy the global approval
capacity and temporarily deny prompts to the legitimate shell. This is an
accepted fail-closed denial-of-service trade-off under the existing same-user
trust gap. The thirty-second lifetime bounds it; the store never evicts an
older request to admit a newer one. Capacity exhaustion emits a content-free
activity event displayed as `approval capacity exhausted`, so the user sees an
explicit failure rather than a silent missing prompt.

## Operational consequences

The engine API, CLI, shell session approval store, tests and fuzz harness migrate
together. An intermediate state that reopens paths or accepts caller identities
must not compile or ship.

## Migration and rollback

The migration replaces selection-bearing `parse_json` calls with wire parsing
and session resolution. Fixed argument-free requests move to named constructors.
Rollback restores the prior API only by reverting the complete commit; partial
rollback is prohibited.

## Validation

- Reject wire fields `inode`, `device`, `content_sha256`, `mode`, workspace
  root and other derived identity.
- Source-check a closed list of domain types that must not derive `Deserialize`,
  `Serialize` or `Clone` as applicable.
- Observe `/proc/self/fd` returning to baseline after deny, cancel, expiry sweep
  and peer disconnect, and enforce per-peer and global caps.
- Prove capacity is reserved before resolution, a rejected request opens no
  descriptor, resolution failure releases its reservation, no pending request
  is evicted, and the shell activity projection reports capacity exhaustion.
- Swap or mutate a selected file between resolution and execution and fail the
  same-descriptor `fstat` comparison.
- Reject replayed, wrong-preview and mutated decisions.
- Prove CLI and model writes use the same resolver and emit their distinct
  origin tags without content or clear paths in audit.
- Fuzz `ToolRequestWire` parsing and resolution through bounded synthetic
  sessions.
