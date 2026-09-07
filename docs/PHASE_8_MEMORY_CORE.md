# Phase 8 closed memory core

Status: complete for the inactive type and validation checkpoint on 2026-09-07.

## Implemented boundary

- Exactly five serialized memory classes distinguish session context,
  temporary memory, user-approved durable memory, project knowledge, and
  system history.
- Nine fixed lifecycle operations map to distinct default-deny capabilities,
  including a separate explicit enable operation.
- The first create draft fixes schema, durable class, user-preference purpose,
  user scope, until-deleted retention, and user-controls-only consumption.
- Validation accepts only non-empty, bounded, explicitly user-authored UTF-8
  values and rejects model output, tool output, context observations, and
  third-party content.
- Record identifiers are bounded opaque tokens and cannot contain paths or
  whitespace.
- The inactive lifecycle is monotonic: proposed, approved, committed, deleted.
  It cannot skip approval, recommit a deleted record, or move backwards.

## Inactive by construction

This checkpoint adds no `ToolRequest` variant, engine route, approval surface,
store, key, filesystem access, service, shell projection, model recall, or
production policy rule. The new capabilities therefore remain unreachable and
default-deny. Storage and public activation begin only in later checkpoints
after their separate evidence passes.

## Deterministic evidence

Unit tests cover stable class serialization, operation-to-capability mapping,
default deny, the exact valid first-slice shape, source/provenance rejection,
size and identifier bounds, schema/class/operation/consumer widening, and every
allowed or forbidden lifecycle transition.

This is type-level and deterministic evidence only. It makes no encryption,
durability, deletion, migration, installed-service, shell, or release claim.
