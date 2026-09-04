# Phase 5 exit audit

Status: complete on 2026-09-04 under accepted ADR-0020. This remains pre-alpha
core infrastructure, not a user-facing autonomous agent or desktop.

## Implemented boundary

- Plans are non-empty, contain at most 16 typed steps, and use bounded
  runtime-supplied plan/correlation identities plus derived request identities.
- Step and request identities are unique. Dependencies may reference only
  unique earlier steps, excluding forward edges, self edges, and cycles.
- Capabilities are derived from `ToolRequest`; plans and models cannot supply
  capability, resource, policy, approval, sandbox, executable, or privilege.
- Phase 4 model proposals may enter only through the narrow bridge for six
  argument-free native reads after per-turn eligibility is rechecked. The
  bridge cannot represent files, writes, services, privilege, Bash, or generic
  execution.
- Every attempted step re-enters the existing engine's policy, one-use approval,
  execution, verification, and audit path. Approval tokens remain private to
  the orchestrator.
- Execution is sequential. A dependency must be `verified`; denial, expiry,
  cancellation, execution failure, or verification failure blocks dependent
  work.
- A monotonic lifecycle prevents `verified` before the verifying state and
  prevents any transition after a terminal result.
- Plan/step states and the final aggregate join the engine's hash-chained audit
  without prompt, reasoning, token, output, file content, service name, process
  data, or caller-selected capability.

## Truthful outcomes

The terminal schema distinguishes verified, denied, cancelled before start,
cancelled after start, execution failed, verification failed, blocked, and
indeterminate. A plan is complete only if every step is verified. Verified
effects followed by non-success become partial completion; explicit uncertainty
dominates as indeterminate.

The readable report is code-generated from those values. It names verified
counts, verified effects, not-started and uncertain counts, then gives each
step's code-derived capability and exact terminal/retry/rollback category. A
model cannot author or rewrite the authoritative status.

No automatic retry or rollback exists. Retry means a new plan with fresh policy
and approval; indeterminate effects prohibit it, while failed effectful work
requires manual review. No current effectful tool has a registered inverse, so
the report truthfully says no rollback is available.

## Cancellation and recovery limits

Cancellation before a step or during approval consumes pending approval, starts
nothing, and prevents later steps. The current engine calls are synchronous and
bounded; they do not expose mid-call interruption. If cancellation arrives
while one is running, it is observed only after that call returns and then stops
later steps. The schema preserves a cancelled-after-start category for a future
executor that can prove that state, but this phase does not claim such an
interruptible executor.

Orchestration state and its audit are in memory. There is no crash resume or
durable recovery claim. The report always states that resume is unsupported;
durable recovery requires a separate ADR.

## Evidence

- Focused orchestration unit tests cover plan/identifier bounds, duplicates,
  forward/self dependencies, static capability derivation, monotonic states,
  eligibility rechecking, duplicate model intents, approval, denial,
  cancellation, dependency blocking, verification failure, audit redaction and
  idempotence, partial/indeterminate aggregation, retry/rollback disposition,
  and readable output.
- `core/blossom-core/tests/orchestration_flow.rs` exercises the public API end to
  end with the real `BlossomEngine`: fresh approval on every step, exact-once
  execution, approval expiry with zero executions, dependent blocking,
  verification failure, final reporting, and audit-chain verification.
- Existing Phase 1-4 request, policy, approval, native capability, sandbox,
  privileged, provider, gateway, and audit tests remain part of protected CI.

## Exit decision

The Phase 5 exit criterion is satisfied: no command dispatch, provider response,
exit status, or tool return can by itself produce plan success. Only a complete
set of code-owned successful verifications can produce `completed`, and every
other state is reported without hiding partial or uncertain work.

Phase 6 has not begun. There is no graphical shell, autonomous loop, generic
Bash, new capability, automatic retry/rollback, durable planner, or background
execution in this phase.
