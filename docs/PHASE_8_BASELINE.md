# Phase 8 memory and personalization baseline

Status: active; ADR-0024 is accepted for the first bounded slice.

Phase 8 begins from the completed typed context boundary in Phase 7. Its goal
is to make memory useful without turning transient activity, model inference,
or security history into invisible durable profiling.

## Ordered implementation checkpoints

1. Review and accept ADR-0024 for closed memory classes and explicit durable
   notes. Complete.
2. Add inactive closed memory types, fixed operation schemas, lifecycle states,
   and deterministic validation tests. Keep durable memory disabled by default.
   Complete; evidence is recorded in `docs/PHASE_8_MEMORY_CORE.md`.
3. Add the encrypted per-user store, external key boundary, restrictive
   permissions, crash-safe writes, and corruption tests without activating a
   public request. Implemented for local deterministic evidence; dependency
   review and remaining production limitations are recorded in
   `docs/PHASE_8_ENCRYPTED_STORE.md`.
4. Route exact create and edit previews through existing policy, once-only
   approval, verification, and content-minimized audit paths. Complete.
5. Add fixed inspect, delete, export, disable, and retention operations with
   verified truthful outcomes. No generic database or query interface.
   Complete.
6. Add a separately reviewed bounded recall projection. Memory is data, never
   authority, approval, identity proof, policy, or current-state verification.
   Complete under ADR-0025.
7. Add narrow shell controls only after the service boundary passes. QML gets
   no storage, key, filesystem, or generic memory authority. The fixed
   shell-facing projections are complete; the existing public request registry
   remains inactive for this slice.
8. Produce deterministic, adversarial, restart, corruption, migration,
   deletion, encryption, and real installed Linux evidence. Deterministic and
   local installed probes pass; the real x86-64 workflow is pending.
9. Perform an independent exit audit and update the roadmap only after
   every required lifecycle control and evidence gate passes.

## First slice

```text
explicit user-authored note
  -> fixed durable-memory capability and disabled-by-default policy
  -> exact value, purpose, scope, retention, and consumer preview
  -> once-only approval bound to the complete request
  -> encrypted atomic per-user persistence
  -> post-write verification and content-free audit outcome
  -> fixed inspect/edit/delete/export/disable controls
```

## Non-goals

This baseline adds no automatic memory, transcript storage, model-authored
facts, background capture, embeddings, semantic search, importance scoring,
cross-project recall, cloud sync, sharing, telemetry, context-source history,
audit reuse, behavioral profiles, or generic storage access.

## Exit evidence

- Accepted ADR-0024 and closed class/operation schemas.
- Durable memory disabled by default with exact approval on every create/edit.
- Encrypted, permission-restricted, crash-safe persistence with separated keys.
- Complete inspect, edit, delete, export, disable, and retention controls.
- Explicit failure states and truthful verification for every lifecycle action.
- Adversarial proof against replay, substitution, silent promotion, plaintext
  residue, corruption, migration failure, hidden recall, and authority reuse.
- Narrow shell projection, if included, with no generic memory or key access.
- Real installed target-Linux evidence and protected regression checks.
- Independent evidence document preserving distribution, backup, recovery,
  hardware, and release limitations.

Phase 8 exits only when no durable memory can be created invisibly.
