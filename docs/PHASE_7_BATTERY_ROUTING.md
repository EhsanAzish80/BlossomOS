# Phase 7 battery request routing

Status: implemented as Phase 7 checkpoint 4 on 2026-09-07. This checkpoint
routes the fixed battery request through the existing engine boundary. It does
not expose battery data to QML, a model, or a generic context API.

## Closed request and policy

The only new request is `system.battery.summary` with an empty argument object.
Unknown arguments, including caller-selected device identifiers, are rejected
before policy evaluation. The request derives exactly
`system.read:battery.summary`; the capability remains denied when no explicit
policy rule exists.

An `ask` decision issues the existing request-bound, expiring, once-only
approval token. The provider is not contacted before approval. An `allow`
decision enters the same native-read route without creating a command or
executor fallback.

## Verification and audit

The engine independently verifies the provider observation against the fixed
source, schema version, lifetime, timestamp freshness, percentage range, and
serialized-size bound. A provider error or invalid observation cannot become a
verified success.

The hash-chained audit records request, capability, decision, approval state,
fixed source, schema version, present-or-absent status, provider failure class,
and verification result. It deliberately omits percentage, charging state,
observation timestamp, device identity, and history. The CLI's audit renderer
supports only those content-minimized events.

## Evidence

Local verification passed:

- 12 focused battery tests covering parsing, scope rejection, default deny,
  ask/allow routing, exact approval, freshness and schema verification,
  provider failure, audit redaction, and zero executor fallback;
- the full workspace suite outside the restricted socket sandbox, including
  200 `blossom-core` unit tests and all integration and component suites;
- `cargo clippy --locked --workspace --all-targets -- -D warnings`; and
- repository policy and diff checks.

The initial restricted full-suite run reached 187 passing core tests, while 13
existing model-adapter tests could not bind loopback sockets and reported
`Operation not permitted`. The unchanged suite passed with the required local
socket permission. This was an environment restriction, not a product failure.

The final Linux x64 Quality run for the signed checkpoint is recorded after the
remote gate completes.

## Remaining boundary

The next checkpoint is the narrow shell projection. Until that separately
reviewed boundary passes, the battery request is not QML- or model-callable.
Installed present-battery and valid no-battery evidence remain first-slice exit
gates.
