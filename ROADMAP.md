# Blossom OS Roadmap

Each phase has an exit gate. Later work must not be used to postpone security or
test requirements from an earlier phase.

## Phase 0: Preserve and baseline

Status: in progress.

- Preserve the original prototype unchanged in Git and tag it.
- Record which features are implemented versus aspirational.
- Connect the empty remote without pushing until the initial history is reviewed.
- Select an open-source license through an ADR.
- Add formatting, linting, tests, CI, dependency policy, and security scanning
  appropriate to each language as it is introduced.
- Decide branch and release policy before accepting external contributions.

Exit: clean local history, approved foundation documents, license decision, and
reviewed first push.

## Phase 1: Deterministic security vertical slice

- Define versioned IPC and tool/capability schemas.
- Implement broker, deny-by-default policy engine, approval state machine,
  sandboxed diagnostic executor, verification, and audit records.
- Build the smallest approval and activity UI needed to exercise the path.
- Use a fixed harmless diagnostic; do not integrate an LLM.

Exit: end-to-end tests prove request -> policy -> approval -> execution ->
verification -> audit, including denial and failure paths.

## Phase 2: Capability and sandbox foundation

- Add narrowly scoped file, application, process, and service-read tools.
- Evaluate and select sandbox technologies by ADR.
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
