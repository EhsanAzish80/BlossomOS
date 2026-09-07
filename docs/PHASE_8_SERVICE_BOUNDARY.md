# Phase 8 durable-memory service boundary

Status: complete for the closed service, lifecycle, recall, and shell-projection
checkpoint on 2026-09-07.

## Mutation boundary

The service begins disabled. Enabling and every lifecycle operation use distinct
capabilities. Create, edit, and delete can proceed only when policy returns
`Ask`; even an accidental `Allow` rule cannot bypass their exact once-only
approval. The preview digest binds request ID, random record ID, operation,
exact value where applicable, purpose, scope, retention, consumer, approval
mode, and expiry.

Denial, cancellation, expiry, replay, service replacement, request mutation,
record conflict, storage failure, or post-write verification failure cannot
produce a verified result. Create accepts only a user-authored value. Edit uses
optimistic version binding; delete verifies the encrypted replacement before
reporting success.

## Lifecycle controls

Fixed methods provide inspect, edit, delete, JSON export, disable, and
until-deleted retention confirmation. There is no caller-selected path, key,
cipher, table, serializer, query language, command, backend, or generic memory
operation. Disable immediately blocks writes, inspection, export, and recall;
it does not silently delete existing encrypted records.

## Recall and shell projection

ADR-0025 limits recall to eight records and 4 KiB of combined values. The
projection is explicitly marked `data_only_not_permission`; it cannot grant
authority or bypass policy and approval. Shell-facing structures contain only
the enabled status and count, fixed record display fields, or the exact
approval preview. They expose no filesystem or cryptographic authority. The
production shell request registry remains unchanged, so memory cannot be
invoked invisibly by the model or an existing client.

## Audit evidence

Mutation audit events contain sequence, request ID, operation, a record-ID
digest, outcome, previous digest, and event digest. Values are excluded. Tests
prove chain verification and mutation detection and verify that private marker
content never appears in serialized audit output.

## Deterministic evidence

Focused tests cover disabled and default-deny behavior, exact binding, replay,
denial, cancellation, expiry, service loss, policy-bypass resistance, create,
edit, inspect, export, retention, bounded recall, verified delete, disable,
content minimization, audit integrity, and fixed shell serialization.
