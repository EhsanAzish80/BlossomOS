# Phase 7 network connectivity core

Status: implemented as the inactive core portion of ADR-0023 on 2026-09-07.
Native provider, request routing, audit, shell projection, and installed evidence
remain separate checkpoints.

The closed registry now contains `system.network.connectivity`, which derives
the singleton capability `system.read:network.connectivity`. Production policy
still denies the capability by default, and no public tool request or provider
route is active in this checkpoint.

The version-1 observation contains only one closed value: `offline`, `local`,
`limited`, `online`, or `unknown`. The validator requires the fixed source,
schema, lifetime, freshness window, present value, and response bound. Absence
is rejected because a missing or unavailable NetworkManager service must remain
an error rather than fabricated offline state.

Tests prove every closed value, default deny, fixed registry metadata, stale and
future rejection, schema and lifetime rejection, absence rejection, and that
serialized output contains no network identifiers. Strict workspace Clippy and
repository checks pass.

This checkpoint deliberately does not add a request parser entry. Activating a
request before the fixed provider and content-minimized audit path exist would
create a partially routed capability. The next checkpoint adds the provider;
request activation follows atomically with policy, verification, and audit.
