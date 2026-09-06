# Phase 6 exit audit

Status: incomplete on 2026-09-06. The architecture, authority boundary,
installed compatibility, and fixed ARM64 approval slice have substantial
passing evidence, but ADR-0021's complete adversarial UI and recovery matrix has
not yet passed. Phase 6 remains active.

## Passing boundary

- ADR-0021 is accepted and keeps QML as an untrusted presentation client.
- The closed, authenticated, versioned, size-bounded session IPC has hostile
  caller, mutation, replay, expiry, disconnect, replacement, and bus-loss tests.
- Approval tokens remain private, connection-bound, mutation-resistant, and
  one-use. Denial, cancellation, and expiry start nothing.
- Only verified engine results render completed, with correlated redacted
  activity authored from the authoritative audit path.
- Static and native-client checks keep command execution, generic D-Bus, policy,
  token custody, filesystem, network, and privileged authority out of QML.
- ARM64 installed graphical evidence covers preview integrity, pointer denial,
  keyboard Escape cancellation, approve-once verification, no-touch expiry,
  and fail-closed service loss.
- x86-64 installed run `34032625185` passed at signed commit `41a84f7`. It
  verified the pinned Arch production versions, real Qt/Rust wire path,
  installed package tree, DRM access, nested protocol requirements, real
  Hyprland, Quickshell QML/plugin loading, and fixed service activation.

## Open exit gates

- Exercise close paths other than Escape and prove every unconsumed approval is
  cancelled without execution.
- Exercise default/global-key and rapid-overlay replacement behavior, including
  a complete keyboard-only approve and deny flow.
- Exercise assistive-technology naming, focus order, activation, and outcome
  announcements against the installed surface.
- Decide and test the durable audit behavior expected across service or bus
  restart. Current in-memory activity loss is explicit and must not be presented
  as recovery.
- Re-run the protected Phase 1-5 regression, lint, dependency, secret, and
  CodeQL checks at the final Phase 6 commit and verify the remote results.

## Decision

The x86-64 installed compatibility gate is satisfied. Phase 6 itself is not yet
complete because the remaining ADR-0021 interaction and recovery requirements
above have no authoritative evidence. No production gate is activated by this
audit.
