# ADR-0025: Bounded durable-memory recall

- Status: Accepted
- Date: 2026-09-07
- Accepted: 2026-09-07 under the project-owner directive to complete the reviewed Phase 8 boundary
- Owners: Project maintainers

## Context

ADR-0024 permits exact, user-approved durable notes but requires a separate
decision before those records may influence a later interaction. Recall can
otherwise become an undeclared authority channel, silently widen data access,
or turn stale personal data into current fact.

## Decision

The first recall slice is a fixed read-only projection of at most eight durable
notes and at most 4 KiB of combined values. It is available only when durable
memory is explicitly enabled and the distinct `memory.durable:recall`
capability is allowed. A limit overflow fails closed instead of silently
returning an unmarked partial value.

Every projection is marked `data_only_not_permission`. Recalled content is
untrusted user data: it cannot grant a capability, approve an operation,
establish identity, change policy, verify present state, select a consumer, or
cause another durable write. Recall performs no ranking, semantic search,
embedding, inference, automatic refresh, network access, or cross-class query.

The user-control shell receives only fixed summary, record, and approval
projections. It receives no store path, key material, cipher parameters,
database/query interface, filesystem handle, or generic memory capability.

## Consequences

Recall is intentionally small and chronological. A caller must treat values as
possibly stale or incorrect and obtain fresh authority through the ordinary
policy and approval path. Disabling memory blocks recall immediately without
silently deleting records. Delete remains a separate exact approved mutation.

## Validation

Completion requires deterministic tests for disabled and denied recall, record
and byte bounds, the non-authority marker, no policy bypass, fixed shell
serialization, and installed Linux evidence covering encrypted persistence,
restart, recall, export, edit, verified delete, and content-free audit.
