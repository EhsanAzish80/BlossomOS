# Phase 5 orchestration-audit checkpoint

Status: implemented under ADR-0020. Phase 5 remains active.

The typed orchestrator now writes plan-level events into the same in-memory,
hash-chained `AuditLog` used by its `BlossomEngine`:

- one plan-accepted event with plan/correlation identifiers and bounded count;
- the initial validated state and every subsequent state for each step, with
  only plan/step identifiers and the code-derived capability; and
- one final aggregate with typed outcome and bounded counters.

The final event is written once even if a caller reads the terminal result more
than once. The projection contains no prompt, reasoning, approval token, tool
output, file content, process data, service name, or caller-provided capability.
Existing per-request policy, approval, execution, verification, and redacted
result events remain in the same chain between plan transitions.

Tests verify chain integrity, required event presence, terminal-event
idempotence, and absence of output content and token fields. This in-memory log
does not claim crash durability or resume support; ADR-0020 requires a separate
decision before durable orchestration recovery.
