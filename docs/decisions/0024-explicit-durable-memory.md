# ADR-0024: Explicit user-approved durable memory

- Status: Accepted
- Date: 2026-09-07
- Accepted: 2026-09-07 after project-owner review of the Phase 8 first slice
- Owners: Project maintainers

## Context

Phase 8 introduces memory and personalization. Durable memory is a materially
different trust boundary from transient model context: it survives a request,
can affect later behavior, and may reveal interests, relationships, routines,
projects, or sensitive facts. A helpful-looking automatic memory feature can
therefore become invisible surveillance, an undeclared source of authority, or
a persistence channel for untrusted content.

The phase must distinguish transient execution data from durable user data and
must make every durable transition visible, attributable, reversible, and
testable. Existing context observations, audit records, conversation text,
tool output, files, clipboard data, notifications, window titles, and model
inferences are not permission to create memory.

## Decision

### Closed memory classes

Blossom recognizes exactly five memory classes:

1. `session_context`: bounded to the active session and removed when it ends;
2. `temporary_memory`: explicitly created with a fixed expiry and never
   promoted automatically;
3. `durable_memory`: created only from a user-authored value after exact
   approval;
4. `project_knowledge`: imported only from an explicitly selected project
   scope under a separately reviewed capability; and
5. `system_history`: security and operational records written only by their
   existing code-owned producers, never repurposed as personalization memory.

Classes cannot be converted, merged, copied, or extended implicitly. Each
class has its own schema, capability, storage namespace, retention rule,
consumer allowlist, and deletion behavior. Search across classes is prohibited
unless a later ADR defines a fixed projection.

### First fixed slice

The first Phase 8 slice permits only an explicit durable note. A request
contains one user-authored UTF-8 value and no model-derived additions. The
service displays the exact value, purpose, scope, retention rule, and allowed
consumers before issuing a once-only approval. Denial, cancellation, expiry,
service replacement, or any mismatch produces no record.

The note schema contains only a random record identifier, schema version,
creation time, last user-edit time, fixed scope, fixed retention mode, and the
approved value. The model cannot select the capability, approve the write,
change retention, suppress the preview, or mark its own output as user-authored.

The first slice has no automatic capture, summarization, inference, importance
score, embedding, background ingestion, cross-project recall, sync, sharing,
telemetry, or network transport. Session context, tool results, context-source
values, files, audit detail, and third-party content cannot be stored through
this operation.

### User control

The durable-memory service remains disabled until a user enables it through an
explicit control. Enabled does not mean pre-approved: every create and edit
still requires an exact preview and once-only approval. Inspect, delete,
export, disable, and retention controls are mandatory before the first slice
may be activated. Disabling prevents new writes and recall; it does not silently
delete records. Delete and export have truthful completion results and may not
claim success before verification.

The service exposes fixed operations rather than a generic database API.
Callers cannot supply a database path, query language, table, key, encryption
parameter, consumer, serializer, or backend command.

### Storage and encryption

Durable records use a code-owned per-user store with restrictive permissions,
authenticated encryption, a versioned envelope, and keys kept outside the
record database. Plaintext values never enter logs or the existing security
audit. Backups, key recovery, export encryption, and key-provider selection
remain inactive until separately specified and tested.

Temporary files, crash reports, migrations, and deleted pages must not leave
recoverable plaintext copies under Blossom's control. If the key provider,
store integrity, schema, authorization, or durability verification fails, the
operation fails closed without a plaintext fallback.

### Recall and authority

Recall is a separate capability from write. The first storage checkpoint does
not expose durable notes to the model or shell. A later reviewed recall slice
must use a bounded typed projection, identify that memory may be stale or
incorrect, and never treat a remembered value as permission, approval,
identity proof, policy, or verified present state.

## Alternatives considered

### Automatically remember useful conversation facts

Rejected. Usefulness is subjective, model classification is not consent, and
automatic extraction hides both persistence and future influence.

### Store conversation transcripts as memory

Rejected. Transcripts combine unrelated purposes and third-party content, make
retention hard to understand, and greatly widen breach and prompt-injection
impact.

### Use audit or system history as personalization data

Rejected. Security evidence has a different purpose, schema, access boundary,
and retention obligation. Reuse would violate content minimization and create
an undeclared personal history.

### Begin with embeddings and semantic search

Deferred. Embeddings introduce derived sensitive data, deletion verification,
index consistency, model provenance, and ranking risks before basic lifecycle
controls have been proved.

## Security and privacy consequences

Every durable transition requires explicit user action and is bound to the
previewed value and metadata. Same-user callers and model output remain
untrusted. Replay, approval substitution, value mutation, scope widening,
class conversion, stale previews, rollback, malformed ciphertext, missing
keys, partial writes, and deletion failures must fail closed.

Metadata still reveals that a record exists and when it changed, so identifiers
are random and audit detail is content-free. Encryption reduces exposure of a
copied store but does not protect plaintext while an authorized process is
using it. Process isolation and consumer allowlists remain required.

## Operational consequences

The implementation needs a native per-user service, a fixed storage schema,
key-provider integration, crash-safe transactions, deterministic clocks and
keys for tests, migrations, and installed Linux evidence. No production
service or shell control is enabled merely by accepting this ADR.

## Migration and rollback

The first slice is additive and disabled by default. Rollback disables create,
edit, and recall before removing code. An incompatible schema change requires
a versioned, atomic migration with backup and rollback evidence; it may not
silently discard or expose records. Removing an accepted memory class requires
an explicit export-or-delete path.

## Validation

Before the first slice is complete, evidence must prove:

- no durable record can be created, edited, promoted, or retained without the
  exact once-only approval bound to its value and metadata;
- denial, cancellation, expiry, replay, service loss, and mismatches leave no
  record;
- memory is disabled by default and disable blocks create and recall;
- inspect, edit, delete, export, retention, and verified outcomes work through
  fixed typed operations;
- plaintext never enters logs, audit, temporary files, migrations, or fallback
  storage;
- encryption, permissions, key separation, atomic durability, corruption, and
  deletion behavior pass adversarial tests;
- system history, context observations, transcripts, tool results, files, and
  model output cannot enter durable memory through the first operation;
- recall cannot grant authority or bypass policy and approval; and
- protected repository checks and installed target-Linux evidence pass.
