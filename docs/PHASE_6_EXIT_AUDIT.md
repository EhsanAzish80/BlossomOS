# Phase 6 exit audit

Status: complete on 2026-09-07. The architecture, authority boundary,
adversarial protocol suite, installed x86-64 interaction matrix, and protected
repository checks have authoritative passing evidence. This closes Phase 6's
reviewed boundary without activating a production gate.

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
- x86-64 installed run `34102303699` passed at signed commit `b3bd89e`. It
  verified the pinned Arch production versions, real Qt/Rust wire path,
  fixed Bubblewrap networkless executor, installed package tree, DRM access,
  nested protocol requirements, real Hyprland, Quickshell QML/plugin loading,
  and fixed service activation.
- The same installed run exercised compositor close and Escape cancellation,
  keyboard-only denial and approve-once execution, rapid ceremony replacement,
  and post-decision refresh. Every unapproved path started nothing; approval
  reached only the fixed `/usr/bin/uname -s` operation and ended verified.
- Standard Qt and Blossom AT-SPI probes verified accessible naming, all 14
  fixed security fields, safe initial focus, keyboard activation, and announced
  denied and verified outcomes against the installed surface.
- Service replacement, bus loss, and shell restart tests invalidate pending
  previews and start nothing. ADR-0021 deliberately specifies no durable
  activity recovery: a service restart produces an empty projection. The tests
  prove this fail-closed non-recovery behavior rather than claiming persistence.
- Quality, prior-phase regressions, dependency review, secret scan, and CodeQL
  passed on the signed implementation commit. Required checks on the final
  documentation commit remain the authoritative remote merge gate.

## Evidence limits outside Phase 6

- This is Arch x86-64 userspace evidence on one trusted Intel GPU runner, not
  broad GPU, display, input-device, kernel, firmware, or hardware support.
- It is not installer, ArchISO, upgrade/rollback, distribution-image, or release
  readiness evidence.
- Durable activity persistence would require a separate reviewed ADR; it is not
  part of the accepted Phase 6 contract.

## Decision

Phase 6 is complete at the accepted ADR-0021 boundary. The installed x86-64
interaction gate is satisfied, the security invariants remain fail closed, and
the remaining limitations are later product and release work. No production
gate is activated by this audit.
