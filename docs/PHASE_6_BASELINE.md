# Phase 6 Blossom Shell baseline

Status: complete on 2026-09-07. ADR-0021 is accepted. The closed Rust protocol,
unprivileged engine bridge, redacted activity projection, and Linux session
D-Bus adapter are implemented. The first narrow graphical surface, ARM64
interaction evidence, and x86-64 installed interaction evidence are recorded.
The exit decision and evidence limits are in the exit audit.

Phase 6 begins from the completed Phase 5 orchestration boundary. Accepted
ADR-0021 defines the shell as an untrusted presentation client and selects a
narrow per-user D-Bus interface, service-authored approval ceremony, and
truthful activity projection.

## Fixed first slice

```text
request fixed /usr/bin/uname -s diagnostic
  -> policy decision
  -> exact service-authored preview
  -> approve once or deny
  -> existing sandbox execution
  -> existing verification
  -> bounded readable activity
```

This baseline adds no capability, command argument, model prompt, generic plan,
file access, privileged operation, or graphical implementation.

## Implementation order after ADR acceptance

1. Freeze the versioned session IPC schema and test bindings. The first closed
   Rust schema and real private-bus transport tests are implemented; installed
   wire evidence passes in final x86-64 run `34102303699`.
2. Implement the unprivileged Rust session service over the existing engine.
   The connection-bound pending state and fixed-diagnostic engine bridge are
   implemented. They use the existing engine audit path, and a bounded closed
   activity projection now exposes only correlated lifecycle categories. The
   Linux adapter authenticates the real D-Bus sender as the same non-root UID.
3. Prove hostile-client, replay, expiry, disconnect, and service-loss behavior.
   The Linux-only adapter subscribes to `NameOwnerChanged` before exporting its
   object or requesting its well-known name. Loss of a client unique name
   cancels that client's pending approval through the existing audited engine
   path. Subscription failure, stream loss, poisoned shared state, or audited
   cancellation failure terminates the service path. The adapter remains behind
   an inactive production feature. A private-bus test uses the real service
   engine and a counting executor to prove that a second same-UID peer, preview
   mutation, decision replay, and cancellation replay cannot start additional
   execution. Further private-bus tests prove a pending preview is invalid while
   its service is absent, remains invalid in a replacement service instance,
   and starts nothing after session-bus loss. Installed compositor-close,
   Escape, focus, and keyboard decision behavior passes in run `34102303699`.
4. Add a minimal approval and activity surface for the fixed slice. Quickshell
   remains the pinned desktop presentation runtime; the security ceremony uses
   a dedicated standard Qt host so the complete control tree reaches AT-SPI.
   The required narrow native Qt/QML transport plugin is implemented and built
   in CI. It compiles in only the four fixed service calls and five UI-facing
   operations; it has no generic D-Bus, process, file, network, policy, token,
   execution, or audit authority. The actual narrow QML surface is implemented.
   ARM64 evidence covers the development sequence, and the x86-64 installed
   adversarial interaction and accessibility matrix passes.
5. Package the pinned Hyprland, Quickshell, D-Bus, and Blossom service boundary.
   The first stable Arch x86-64 compatibility set is fixed to Hyprland
   `0.56.2-2`, Quickshell `0.3.1-1`, systemd `261.2-1`, and dbus-broker `37-3`.
   An inactive hardened user unit, session activation metadata, closed lock,
   and CI drift validator are implemented. Installed compatibility passes in
   x86-64 run `34102303699`.
6. Produce installed Arch userspace evidence and an independent exit audit.
   The disposable nested-compositor harness produced authoritative passing
   x86-64 run `34102303699` on a trusted Linux GPU runner. Its
   evidence and limitations are documented in
   `docs/PHASE_6_INSTALLED_EVIDENCE.md`; the independent completion decision and
   evidence limits are in `docs/PHASE_6_EXIT_AUDIT.md`.

## Exit evidence required

- Accepted Phase 6 architecture ADR.
- Closed, authenticated, versioned, bounded IPC with adversarial tests.
- Exact immutable approval preview with private one-use token custody.
- Denial and cancellation that start nothing.
- Verified result and correlated redacted activity display.
- No QML-to-command, QML-to-helper, or caller-selected authority path.
- Pinned real Hyprland + Quickshell installed evidence on Arch userspace.
- Passing prior-phase regression, lint, dependency, secret, and CodeQL checks.
- Documentation distinguishing installed evidence from hardware, ArchISO,
  installer, distribution, and release-readiness proof.
