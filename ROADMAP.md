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

Status: complete. ADR-0009 selects the system D-Bus, polkit, idempotency, audit,
and hardening boundary. ADR-0010 fixes the first operation to a try-restart of
the already-running `bluetooth.service`. The single system-bus method, fixed
native adapters, durable state, packaging boundary, and interactive client are
implemented. They remain pre-alpha and are not installed by this repository.

- [x] Design a minimal typed helper and polkit policy boundary.
- [x] Add the fixed low-complexity Bluetooth try-restart operation with approval
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
  - [x] Add the interactive CLI path with exact fixed-operation preview,
    once-only approval, secure random idempotency, non-TTY denial, independent
    result verification, and correlated readable activity.
  - [x] Wire and package the helper, polkit check, root-owned runtime journal,
    systemd operation, interactive client, double audit, and Linux evidence.
- [x] Complete threat review, negative tests, replay protection, and audit coverage.

Exit satisfied: `docs/PHASE_3_BASELINE.md` records the requirement-by-requirement
evidence and independent no-generic-root-path review. The exit checkpoint passed
protected CI and merged to `main` as `0662e51`.

## Phase 4: Replaceable local AI runtime

Status: active. ADR-0011 defines the accepted provider-neutral local inference
boundary. Implementation begins with closed core types and synthetic fixtures;
private inputs remain blocked pending an accepted endpoint-identity ADR.

- [x] Accept the provider-neutral inference, proposed-tool-intent, cancellation,
  streaming, locality, privacy, and conformance contract.
- [x] Implement closed core types, validation, cancellation, normalized stream
  state, redacted audit projection, and scripted conformance tests.
- [x] Implement the fixed-local, synthetic-only Ollama development adapter.
- [x] Implement the fixed-local, synthetic-only llama.cpp development adapter.
- [ ] Implement the accepted ADR-0012 provider endpoint-identity and packaging
  boundary before either real adapter may receive private or ambient user data.
  - [x] Accept the distinct-UID gateway, Unix peer-credential, isolated-provider,
    and root-owned profile design.
  - [x] Implement the closed synthetic-only gateway framing, request/event
    validation, cancellation binding, and Linux peer-credential primitives.
  - [x] Implement the synthetic-only Unix client and one-request fixture gateway,
    including separate-process proof that peer validation precedes request bytes.
  - [x] Define and validate the closed, CPU-only root-owned provider-profile
    manifest format, canonical expected bytes, artifact/unit digests, fixed
    endpoints, identities, filesystem scope, and resource bounds using only
    synthetic filesystem fixtures.
  - [ ] Implement and package the gateway, static identities, namespace anchor,
    hardened services, closed production profile registry, installed manifests,
    and runtime identity evidence.
  - [ ] Pass adversarial production-path Linux evidence before enabling private
    input.
- [ ] Validate every model output and proposed tool intent against strict,
  code-owned schemas before it reaches the broker.
- [ ] Produce controlled-protocol and real-model target-Linux evidence with
  external network access disabled.

Arbitrary shell remains a separately reviewed fallback capability in the target
security model; Phase 4 does not implement or expose it.

Exit: deterministic conformance tests pass for both providers, real local-model
operation is verified offline on target Linux, and no provider path bypasses the
broker, policy, approval, verification, or audit boundary. Unauthenticated
loopback endpoints receive no private input.

Checkpoint evidence: `docs/PHASE_4_CORE_CONTRACT.md` and
`docs/PHASE_4_OLLAMA_ADAPTER.md`, and
`docs/PHASE_4_LLAMA_CPP_ADAPTER.md`. Phase 4 remains active; these checkpoints
claim neither an authenticated provider endpoint, private-input support, nor a
real-model test. Cross-provider fixture equivalence, offline real-model
target-Linux evidence, and the endpoint-identity boundary remain unresolved.
ADR-0012 accepts the endpoint-identity and packaging boundary; it is not yet
fully implemented. The closed protocol and credential-primitives checkpoint is
documented in `docs/PHASE_4_GATEWAY_PROTOCOL.md`; the synthetic process boundary
is documented in `docs/PHASE_4_GATEWAY_FIXTURE.md`. The closed manifest validator
and synthetic filesystem evidence are documented in
`docs/PHASE_4_PROVIDER_MANIFEST.md`. No production manifest, registry, identity,
or service is installed, and private input remains blocked.

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
