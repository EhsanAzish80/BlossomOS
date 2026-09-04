# Phase 6 Blossom Shell baseline

Status: active design work. No Phase 6 shell or IPC implementation is claimed.

Phase 6 begins from the completed Phase 5 orchestration boundary. Proposed
ADR-0021 defines the shell as an untrusted presentation client and proposes a
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

1. Freeze the versioned session IPC schema and test bindings.
2. Implement the unprivileged Rust session service over the existing engine.
3. Prove hostile-client, replay, expiry, disconnect, and service-loss behavior.
4. Add a minimal Quickshell approval and activity surface for the fixed slice.
5. Package the pinned Hyprland, Quickshell, D-Bus, and Blossom service boundary.
6. Produce installed Arch userspace evidence and an independent exit audit.

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
