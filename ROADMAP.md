# Blossom OS Roadmap

Each phase has an exit gate. Later work must not be used to postpone security or
test requirements from an earlier phase.

## Phase 0: Preserve and baseline

Status: complete (2026-09-02).

- [x] Preserve the original prototype unchanged in Git and tag it as
  `prototype-pre-agent-architecture`.
- [x] Publish the reviewed initial history to the GitHub `main` branch.
- [x] Record which features are implemented versus aspirational.
- [x] Select Apache-2.0 through accepted ADR-0001 and add the canonical license.
- [x] Add authoritative vision, architecture, security, roadmap, contribution,
  and vulnerability-policy documents.
- [x] Audit the preserved prototype for obvious credentials, unsafe defaults,
  generated artifacts, large binaries, and machine-specific paths.
- [x] Enable GitHub Private Vulnerability Reporting and publish the active link
  in `SECURITY.md`.
- [x] Document branch, review, release, and signing policy.
- [x] Enforce the documented policy for `main` with branch protection.
- [x] Define baseline repository checks, shell linting, prototype smoke tests,
  CI, dependency policy, secret scanning, and Python CodeQL analysis.
- [x] Verify the initial GitHub Actions runs succeed on the default branch.

Evidence and unresolved items are recorded in `docs/PHASE_0_BASELINE.md`.

Exit: clean published history, approved internally consistent foundation
documents, accepted license, accurate implementation inventory, private security
reporting, branch/release policy, and explicitly defined baseline quality gates.

## Phase 1: Deterministic security vertical slice

Entry gate satisfied. Begin on a separate scoped branch without integrating an
LLM or restructuring the preserved prototype.

Status: complete (2026-09-02). Evidence is documented in
`docs/PHASE_1_SECURITY_CORE.md`.

- [x] Define the initial typed request and tool/capability schemas. Versioned IPC
  transport remains intentionally deferred until a separate process exists.
- [x] Implement broker, deny-by-default policy engine, approval state machine,
  sandboxed diagnostic executor, verification, and audit records.
- [x] Build the smallest interactive terminal approval and activity surface needed
  to exercise the path, including non-TTY denial and audited cancellation.
- [x] Use a fixed harmless diagnostic; do not integrate an LLM.

Exit: end-to-end tests prove request -> policy -> approval -> execution ->
verification -> audit, including denial and failure paths.

## Phase 2: Capability and sandbox foundation

Status: complete (2026-09-02). ADR-0004 defines the accepted capability
taxonomy, expansion order, privacy rules, and per-tool exit evidence. The
completion audit and capability matrix are recorded in
`docs/PHASE_2_BASELINE.md`.

- [x] Define the capability taxonomy and expansion rules before adding tools.
- [x] Add narrowly scoped system, process, file, and service-read tools in the
  order fixed by ADR-0004.
  - [x] `system.read:os.identity` via bounded native os-release parsing.
  - [x] `system.read:uptime` via a bounded native `/proc/uptime` read.
  - [x] `system.read:memory.summary` via a bounded native `/proc/meminfo` read.
  - [x] `system.read:storage.summary` via native root-filesystem `statvfs`.
  - [x] `process.read:self` via native calling-process identity APIs.
  - [x] `process.read:list` with once-only approval, same-effective-user scope,
    bounded `/proc/<pid>/status` reads, and redacted audit counts.
  - [x] One approval-gated user-selected exact UTF-8 file read using ADR-0005
    containment and a 64-KiB bound.
  - [x] One approval-gated workspace-contained atomic file creation using
    retained descriptors, fixed `0600`, and no-replace publication.
  - [x] One approval-gated exact systemd service-status observation using fixed
    native D-Bus calls and a redacted audit scope (ADR-0008).
- [x] Evaluate and select confinement technologies per resource class by ADR or
  an accepted extension of an existing profile.
  - [x] Exact-file reads: Linux `openat2` no-symlink resolution plus a retained
    descriptor and identity revalidation (ADR-0005).
  - [x] Workspace file creation: retained directory descriptors, no mount or
    symlink traversal, and unnamed-inode atomic no-replace publication
    (ADR-0007, superseding ADR-0006).
  - [x] Exact service-status reads: fixed systemd system-bus calls for one
    approval-bound `.service` unit, with no listing, loading, mutation, generic
    D-Bus, or subprocess fallback (ADR-0008).
- [x] Keep environment, working-directory, filesystem, network, timeout, output,
  process, and resource controls code-owned: the fixed Phase 1 command uses its
  Bubblewrap profile, while native Phase 2 tools use resource-specific bounds
  and confinement documented in the capability evidence.
- [x] Add property, adversarial, and integration tests for each applicable
  security boundary, including protected target-Linux evidence.

Exit satisfied: every registered tool declares a static capability and passes
its applicable containment tests. See `docs/PHASE_2_BASELINE.md`.

## Phase 3: Privileged operations

Status: active. ADR-0009 selects the system D-Bus, polkit, idempotency, audit,
and hardening boundary. ADR-0010 fixes the first operation to a try-restart of
the already-running `bluetooth.service`. The portable helper state machine is
implemented, but it is not yet a root service and exposes no system-bus method.

- [x] Design a minimal typed helper and polkit policy boundary.
- [ ] Add the fixed low-complexity Bluetooth try-restart operation with approval
  and verification.
  - [x] Define the closed shared request/result protocol, normalized digest, and
    independent result verifier without registering or exposing the tool.
  - [x] Implement the portable helper security state machine with independent
    authorization/manager adapters, replay-safe journal transitions, truthful
    interruption outcomes, verification, and redacted transition events.
  - [x] Add a bounded, durable file-journal backend with root ownership and mode
    checks, no-follow opens, synced transitions, atomic replacement, and
    fail-closed corruption/recovery behavior.
  - [x] Add a bounded, synced, hash-chained helper audit backend that validates
    its complete chain on recovery and fails closed on loss or tampering.
  - [x] Add the native code-owned systemd adapter for only
    `TryRestartUnit("bluetooth.service", "replace")`, including matching job
    completion and changed-invocation observation on target Linux.
  - [x] Add the native fixed-action polkit adapter using only the authenticated
    system-bus sender subject and non-retained interactive authorization.
  - [x] Wire the single exported system-bus method to authenticated sender/UID
    capture and the concrete polkit, journal, audit, and systemd components.
  - [x] Add the reviewed package boundary: hardened systemd unit, exact D-Bus
    activation/policy files, one polkit action, and automated drift validation.
  - [ ] Wire and package the helper, polkit check, root-owned runtime journal,
    systemd operation, interactive client, double audit, and Linux evidence.
- Complete threat review, negative tests, replay protection, and audit coverage.

Exit: independent review confirms there is no generic root command path.

## Phase 4: Replaceable local AI runtime

- Define provider-neutral inference, tool-call, cancellation, and streaming APIs.
- Implement one local provider, then a second provider to prove replaceability.
- Validate all model output against strict schemas.
- Keep arbitrary shell as an explicit fallback capability.

Exit: deterministic conformance tests pass for both providers and offline operation
is verified.

## Phase 5: Planning and verification

- Add intent, plan, capability analysis, approval, execution, verification, and
  truthful-summary states.
- Handle partial completion, rollback opportunities, cancellation, and recovery.
- Treat tool output and retrieved content as untrusted input.

Exit: Blossom never reports success solely because a command was issued.

## Phase 6: Blossom Shell

- Establish Hyprland + Quickshell development environment.
- Build agent panel, approval UI, activity history, launcher, notifications, and
  system status incrementally.
- Keep shell UI separate from authorization and execution services.

Exit: the shell can operate the tested vertical slices without XFCE dependencies.

## Phase 7: Structured system awareness

- Add applications, windows, workspaces, hardware, battery, network, storage,
  services, clipboard, notifications, selected files, and active-project context.
- Prefer native APIs and IPC over screenshots or accessibility automation.
- Apply per-source privacy, lifetime, and capability rules.

Exit: context sources are typed, permissioned, observable, and testable.

## Phase 8: Memory and personalization

- Separate session context, temporary memory, user-approved durable memory,
  project knowledge, and system history.
- Add inspect, edit, delete, export, disable, retention, and encryption controls.

Exit: no durable memory is created invisibly.

## Phase 9: Distribution and updates

- Build Arch packages, services, ArchISO, hardware detection, model selection,
  first-run setup, recovery, rollback, and signed updates.
- Remove insecure legacy defaults and test upgrade paths.

Exit: repeatable VM installation and rollback evidence exists before device claims.

## Phase 10: Public beta hardening

- Threat-model review, dependency audit, fuzzing, privilege and sandbox review,
  signed releases, SBOM, practical reproducible builds, and disclosure process.
- Publish clear limitations and supported hardware.

Exit: release checklist and security review are complete with traceable evidence.
