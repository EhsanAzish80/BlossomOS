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

Status: in progress. ADR-0004 defines the accepted capability taxonomy,
expansion order, privacy rules, and per-tool exit evidence. No Phase 2 tool is
implemented yet.

- [x] Define the capability taxonomy and expansion rules before adding tools.
- [ ] Add narrowly scoped system, process, file, and service-read tools in the
  order fixed by ADR-0004.
- [ ] Evaluate and select confinement technologies per resource class by ADR or
  an accepted extension of an existing profile.
- Enforce environment, working-directory, filesystem, network, timeout, output,
  process, and resource controls.
- Add property, adversarial, and integration tests for the security boundary.

Exit: every registered tool declares capabilities and passes containment tests.

## Phase 3: Privileged operations

- Design a minimal typed helper and polkit policy.
- Add one low-complexity privileged operation with approval and verification.
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
