# ADR-0020: Closed orchestration and truthful outcomes

- Status: Accepted
- Date: 2026-09-04
- Accepted: 2026-09-04 as the explicitly approved Phase 5 prerequisite
- Owners: Project maintainers

## Context

Phases 1 through 3 established typed requests, policy, once-only approval,
bounded execution, verification, audit, and one narrow privileged operation.
Phase 4 added replaceable local inference while keeping every model response
untrusted and non-authorizing. Phase 5 must compose those boundaries without
turning a plan into a generic execution language or reporting success merely
because a step was dispatched.

Multi-step work introduces distinct failure states: a plan can be invalid, a
capability can be denied, approval can expire, execution can fail, verification
can reject an apparent success, cancellation can race with a step, and earlier
steps can remain completed after a later failure. Collapsing those states into
`success` or `failure` would mislead the user and obscure recovery choices.

## Decision

### Trust and authority

The orchestrator is deterministic trusted code. The model, prompt, retrieved
content, plan proposal, tool output, provider metadata, and summary text are
untrusted data. None can:

- construct an executable `ToolRequest` or privileged-helper request directly;
- choose a capability, scope, selected resource, retained descriptor, policy,
  sandbox, executable, argument vector, D-Bus method, or provider endpoint;
- grant, reuse, broaden, or persist approval;
- mark execution or verification successful;
- skip, reorder, insert, retry, or roll back a step after plan acceptance; or
- write authoritative audit state.

Only the code-owned planner adapter may translate a completely validated intent
proposal into a closed plan step. Each step references one registered typed
operation and a resource selection already established by the trusted user or
system boundary. Model-provided paths remain suggestions and never become file
authority.

### Closed plan

The first schema is in-process Rust only and versioned. A plan contains a
runtime-generated plan identifier, a bounded user-request correlation, and an
ordered non-empty set of at most 16 steps. Each step has a runtime-generated
identifier, one closed typed intent, its exact static capability projection,
and a dependency set restricted to earlier step identifiers.

Unknown fields, variants, capabilities, tools, resources, forward dependencies,
self-dependencies, cycles, duplicate identifiers, empty plans, excessive size,
or a mismatch between intent and capability fail before policy or execution.
The initial executor is strictly sequential even when steps are independent.
There is no loop, branch, recursion, variable expansion, interpolation, shell,
generic command, dynamic tool discovery, caller-selected retry, or model-defined
rollback program.

### State machine

Every accepted plan moves monotonically through code-owned states:

```text
proposed -> validated -> capability_analyzed -> awaiting_approval -> executing
         -> verifying -> terminal
```

Policy is evaluated separately for every step immediately before that step can
start. `deny` terminates that step without execution. `ask` requires a fresh
once-only approval bound to the exact request, capability, resource, operation,
plan and step. `allow` is not an approval and cannot be reused for another step.
Non-interactive approval remains deny by default.

Only one step may execute at a time. A later step cannot enter policy until all
of its dependencies have a verified-success outcome. Verification consumes the
actual typed result from the exact execution; dispatch, process exit zero, HTTP
success, D-Bus acceptance, or model assertion alone never establishes success.

### Terminal outcomes and summaries

Every step terminates exactly once as one of:

- `verified`: execution completed and code-owned verification passed;
- `denied`: policy or user denied before execution;
- `cancelled_before_start`: cancellation won and nothing started;
- `cancelled_after_start`: work may have started and no success is claimed;
- `execution_failed`: execution returned a bounded non-success;
- `verification_failed`: execution returned but verification rejected it;
- `blocked`: a dependency or required boundary prevented the step from starting;
  or
- `indeterminate`: the system cannot prove whether the requested effect occurred.

The plan result is `completed` only when every required step is `verified`.
Otherwise it is `partially_completed` if at least one effectful step verified,
`cancelled` if cancellation won before any effect was verified, `blocked` if no
step started, or `indeterminate` whenever any step is indeterminate.

The authoritative summary is constructed from these typed outcomes, not from a
model. It states what was verified, what did not start, what may have occurred,
and what remains. A model may later rephrase a separately labelled explanation,
but cannot alter status, omit uncertainty, or create the authoritative result.

### Cancellation, retry, recovery, and rollback

Cancellation is plan-bound and monotonic. It prevents every not-yet-started
step, consumes any pending approval, signals the active provider/executor when
supported, waits only for a code-owned bound, and records whether termination
was observed. Cancellation never converts an unknown effect into success.

Phase 5 performs no automatic retry. A retry is a new runtime-generated plan or
step attempt with new identifiers, fresh policy evaluation, fresh approval when
required, and an explicit relationship to the prior outcome. Non-idempotent or
indeterminate work is never retried automatically.

Rollback is descriptive in the first contract: a tool may expose a code-owned
rollback opportunity and its prerequisites, but the orchestrator does not run
it automatically. Exercising it requires a new typed request through policy,
approval, execution, verification, and audit. “Rollback available” is never
reported as “rolled back.”

Recovery resumes from durable authoritative state only after validating schema,
integrity, plan identity, step identities, and terminal outcomes. The first
implementation is in-memory and therefore does not claim crash recovery.
Durable resume requires a separate ADR before implementation.

### Audit and privacy

Each transition records plan/step correlation, state, capability identifier,
policy category, approval outcome, execution category, verification category,
and terminal result. Audit records omit prompts, model reasoning, file content,
tool output content, credentials, approval tokens, and unnecessary resource
names. Sensitive scopes use existing redacted or digested representations.

An audit write required to authorize or finalize a step fails closed. A missing
terminal audit record cannot be treated as verified completion. Audit evidence
must distinguish “not started,” “started but unverified,” and “verified.”

### Phase 5 implementation order

1. closed plan identifiers, schema, limits, validation, capability projection,
   state transitions, and terminal aggregation;
2. deterministic single-step orchestration over existing typed engine paths;
3. bounded sequential multi-step execution, dependency blocking, cancellation,
   partial completion, retry representation, and rollback opportunities;
4. authoritative summary and orchestration audit projection;
5. adversarial end-to-end tests and an exit audit.

The initial tool catalogue remains the existing typed Phase 1-3 operations. No
new capability, generic Bash, LLM-controlled execution, IPC daemon, graphical
shell, durable recovery, or automatic rollback is introduced merely to satisfy
Phase 5.

## Alternatives considered

### Let the model emit and execute a JSON plan

Rejected. JSON shape validation alone does not establish capability authority,
resource selection, approval, execution truth, or safe composition.

### Treat each tool call independently without a plan state machine

Rejected. This loses dependency, cancellation, partial-completion, recovery,
and truthful aggregate-outcome semantics.

### Automatically retry or roll back failures

Rejected for the initial contract. Effects can be non-idempotent or
indeterminate, and compensating actions have their own permissions and risks.

### Let a model write the final status

Rejected. A fluent summary cannot replace code-owned verification evidence.

## Security and privacy consequences

The design preserves the existing broker and approval boundaries and prevents
plan composition from creating ambient authority. It adds denial-of-service and
privacy risks from oversized or sensitive plans, addressed by closed schemas,
small bounds, minimal capability projection, redacted audit, and sequential
execution. Prompt injection remains able to influence proposals, but not
resource authority, approval, execution policy, or verified status.

## Operational consequences

Plans are deterministic and reviewable but initially less flexible than a
general agent loop. Sequential execution is slower but makes state, cancellation
and evidence unambiguous. In-memory state means process loss terminates the plan
without a resume claim; durable orchestration is deferred.

## Migration and rollback

The orchestration module is additive and cannot replace direct tested
single-request paths until equivalent tests pass. It is introduced without new
capabilities or default runtime activation. Reverting the module returns to the
existing direct typed flows; accepted audit evidence remains readable.

## Validation

Tests must prove:

- every invalid, oversized, cyclic, expanded, mismatched, or unknown plan fails
  before policy and execution;
- model output cannot forge identifiers, capabilities, scopes, approval,
  execution, verification, audit, retry, rollback, or summary status;
- every step re-enters policy and required approval exactly once;
- denial, approval expiry/replay, executor failure, verifier failure, audit
  failure, provider loss, cancellation races, and dependency failure produce
  exact non-success outcomes and start no unauthorized later step;
- partial completion names only verified effects and never collapses to success;
- an indeterminate effect dominates the plan result and blocks unsafe retry;
- summaries are deterministic projections of typed results and preserve every
  uncertainty; and
- existing Phase 1-4 negative tests, strict formatting/clippy, dependency
  review, Gitleaks, CodeQL, and repository checks remain green.
