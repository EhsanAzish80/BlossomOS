# Phase 5 typed-orchestrator checkpoint

Status: implemented as the second ADR-0020 checkpoint. Phase 5 remains active.

`PlanOrchestrator` now drives validated plan steps through the existing
`BlossomEngine` typed entry point. The engine still owns policy, approval,
execution, verification, and its established per-request audit trail.

The integration provides:

- strict sequential dispatch of existing `ToolRequest` values only;
- dependency blocking unless every named predecessor is verified;
- one policy evaluation per attempted step through the existing engine;
- approval tokens retained privately inside the orchestrator and never exposed
  in `OrchestrationEvent`;
- explicit approve-once, deny, and pending-approval cancellation methods;
- prevention of execution before approval;
- verification-derived `Verified`, `VerificationFailed`, and
  durability-`Indeterminate` outcomes;
- exact request binding checks on engine results; and
- deterministic final aggregation only after every step is terminal.

Focused integration tests prove approve-once execution, denial-dependent
blocking, pending cancellation with zero executions, and refusal to convert a
nonzero diagnostic result into plan success. Strict workspace clippy and the
existing engine tests cover the unchanged direct request paths.

This checkpoint does not yet accept model-authored plans, persist orchestration
state, cancel synchronous work after it starts, emit plan-level audit records,
represent retry/rollback opportunities, or provide the final readable summary.
Those remain required before Phase 5 exit.
