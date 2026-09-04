# Phase 5 model-intent to plan checkpoint

Status: implemented under ADR-0020. Phase 5 remains active.

`ValidatedPlan::from_model_intents` is the only Phase 5 bridge from Phase 4
model proposals into orchestration. It accepts normalized `ProposedToolIntent`
values and independently rechecks membership in the trusted per-turn catalogue.

The bridge supports exactly the six argument-free native read intents already
in the Phase 4 model schema: OS identity, uptime, memory summary, root storage
summary, Blossom process identity, and approval-gated process list. It cannot
represent a pathname, file read, file write, service operation, privileged
operation, executable, argument vector, shell, endpoint, policy, approval, or
sandbox configuration.

Plan and correlation identifiers come from trusted runtime inputs. The bridge
derives each request identifier with a domain-separated SHA-256 digest, creates
fixed ordinal step identifiers, rejects empty, excessive, duplicate, and
turn-ineligible intent sets, and chains each step to its predecessor. The plan
validator then derives capabilities from the resulting `ToolRequest` variants.
Provider fields therefore cannot become internal request IDs, dependencies, or
capabilities.

Tests cover a real provider-completion validation followed by translation,
deterministic identifiers, fixed capabilities, conservative dependencies,
eligibility rechecking, and duplicate rejection. The bridge does not invoke a
model, select a catalogue, or execute a plan; those authority decisions remain
outside provider output.
