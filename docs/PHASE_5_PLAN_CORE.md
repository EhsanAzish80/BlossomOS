# Phase 5 closed-plan core checkpoint

Status: implemented as the first ADR-0020 checkpoint. This is not an executing
agent loop and does not complete Phase 5.

`core/blossom-core/src/orchestration.rs` adds an in-process Rust boundary for:

- bounded plan and step identifiers;
- non-empty plans of at most 16 sequentially interpreted steps;
- unique request and step identities;
- dependency edges that can reference only unique earlier steps;
- static capabilities derived only from the existing typed request registry;
- monotonic validated, capability-analysis, approval, execution, verification,
  and terminal phases;
- terminal outcomes that distinguish verified, denied, cancelled before/after
  start, execution failure, verification failure, blocked, and indeterminate;
  and
- an authoritative aggregate that cannot report `completed` unless every step
  has a verified terminal outcome.

The types expose no deserializer for model-authored plans, no executor, no
policy override, no approval token, no caller-supplied capability, no dynamic
tool name, no retry, no rollback action, no generic command, and no persistence.
Model-to-intent translation and actual orchestration remain later protected
checkpoints.

Focused tests cover identifier and step bounds, empty plans, duplicate step and
request identifiers, duplicate/forward/self dependencies, capability derivation,
invalid state transitions, denial/cancellation before execution, prevention of
verified status before verification, indeterminate-result precedence, and
missing terminal evidence.

Phase 5 still requires integration with the existing policy/approval/execution/
verification paths, dependency blocking, plan cancellation, partial completion,
audit projection, truthful readable summaries, adversarial end-to-end tests,
and an exit audit.
