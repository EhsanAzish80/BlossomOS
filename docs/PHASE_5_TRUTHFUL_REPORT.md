# Phase 5 truthful-report checkpoint

Status: implemented under ADR-0020.

`TruthfulPlanReport` is a deterministic projection of a validated plan and its
complete typed terminal outcomes. It reports the plan outcome, verified steps,
verified effectful steps, work that did not start, uncertain steps, and one row
per step containing only its identifier, code-derived capability, outcome,
retry disposition, and rollback disposition.

The report cannot contain model prose or tool output. `completed` requires every
step to be `verified`; an explicit indeterminate outcome dominates the plan.
Verified writes followed by blocked work produce `partially completed`.
Indeterminate work prohibits retry, and failures after an effectful step starts
require manual review. All retries must be new plans with fresh identifiers,
policy, and approval. The current create-only write has no registered inverse,
so the report says that rather than claiming rollback.

Recovery is explicitly `InMemoryOnlyNoResume`. Process loss cannot be presented
as resumable state. The synchronous typed engine accepts cancellation between
steps and while approval is pending; pending cancellation consumes approval and
starts nothing. Once a synchronous operation has started, it runs only to its
existing code-owned timeout/result boundary, after which cancellation prevents
later steps. No mid-call interruption or crash recovery is claimed.

Unit and public integration tests cover fresh approval per step, dependency
blocking, approval expiry, pending cancellation, verification failure,
partial completion, indeterminate precedence, retry safety, rollback honesty,
audit integrity, and deterministic readable output.
