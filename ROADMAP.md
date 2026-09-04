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

Status: complete. ADR-0011 defines the accepted provider-neutral local inference
boundary, ADR-0012 defines the endpoint-identity and packaging boundary, and
ADR-0017 fixes production admission and cancellation semantics. Controlled
synthetic implementation and pinned llama.cpp installed evidence are
substantial. The two pinned x86-64 provider paths and target-Arch
userspace/package/ABI checks now have passing evidence. Private input remains a
non-default package feature; Phase 4 completion is not a release-readiness claim.

- [x] Accept the provider-neutral inference, proposed-tool-intent, cancellation,
  streaming, locality, privacy, and conformance contract.
- [x] Implement closed core types, validation, cancellation, normalized stream
  state, redacted audit projection, and scripted conformance tests.
- [x] Implement the fixed-local, synthetic-only Ollama development adapter.
- [x] Implement the fixed-local, synthetic-only llama.cpp development adapter.
- [x] Implement the accepted ADR-0012 provider endpoint-identity and packaging
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
  - [x] Define the inactive package boundary for persistent non-login identities,
    fixed paths, private namespace anchor, gateway unit, and closed CPU-provider
    unit templates, with repository drift checks and systemd verification.
  - [x] Add the fail-closed gateway process scaffold and Linux separate-process
    evidence for the one fixed synthetic request; release/default startup creates
    no listener while production admission and readiness proof are absent.
  - [x] Add the debug/test-only closed synthetic profile registry and
    deterministic provider-unit renderer, binding exact manifest command,
    environment, filesystem, resource, identity, and rendered-unit digest data.
  - [x] Add fail-closed installed-account resolution and runtime-readiness
    validation that binds the canonical manifest to root-owned runtime, model,
    rendered-unit and account-database descriptor evidence while retaining the
    validated artifact descriptors against path-replacement TOCTOU.
  - [x] Route the two closed synthetic gateway profiles through the actual
    bounded Ollama and llama.cpp adapters, with Linux end-to-end framing,
    credential, request, provider-response, normalization and terminal-event
    evidence.
  - [x] Accept ADR-0013 and migrate the manifest/readiness boundary to closed,
    canonical model artifact sets so every llama.cpp GGUF or Ollama
    manifest/blob consumed by a profile is measured and unknown entries fail
    closed.
  - [x] Accept ADR-0014 and ADR-0015 so canonical profiles bind fixed account
    names rather than target-assigned numeric IDs and measure every executable
    and bundled provider-runtime file as a closed immutable artifact set.
  - [x] Accept ADR-0016 and add the first release-constructible x86-64 llama.cpp
    registry entry plus an offline, deterministic, hash-pinned package-tree
    recipe.
  - [x] Render the gateway service per closed package profile so its sandbox
    exposes only the selected manifest, measured runtime set, and model needed
    for readiness validation; keep the production listener disabled.
  - [x] Make supported-Linux release startup select the sole embedded profile,
    consume installed readiness, and match its effective service UID/GID before
    returning fail-closed without a listener.
  - [x] Accept ADR-0017 for retained-account client eligibility,
    server-derived private request identity, one-request connections,
    cancellation races, and redacted production evidence.
  - [x] Retain the exact account-database bytes used by readiness and authorize
    connected non-root clients by kernel UID plus primary or unique
    supplementary `blossom-ai` membership without reopening account paths.
  - [x] Add the distinct canonical private-inference frame whose wire payload
    cannot select a provider, model, endpoint, classification, path or runtime
    setting; its decoder injects provider/model/private identity from admitted
    code-owned inputs.
  - [x] Bind logical model identity into canonical provider-profile schema v5,
    making the selected embedded and installed-manifest-verified profile the
    only production source for private request decoding.
  - [x] Implement the already-authorized one-request connection state machine,
    including hello-before-input, pipelining rejection, concurrent bound
    cancellation, validated event encoding, bounded I/O and fail-closed
    disconnect/write behavior.
  - [x] Wire that handler into an explicitly package-feature-gated target-Linux
    listener with stale-path refusal, exact socket metadata, boot/process
    identity, peer credentials and retained-snapshot authorization. Default
    builds remain fail closed until installed adversarial evidence enables it.
  - [x] Accept ADR-0018 and implement the boot-scoped, synced, hash-chained,
    content-free operational journal so request-start evidence precedes
    inference and terminal evidence precedes terminal delivery.
  - [x] Implement and package the gateway, static identities, namespace anchor,
    hardened rendered services, closed production profile registry, installed
    manifests, and runtime identity evidence.
    - [x] Complete that package and runtime-identity evidence for the pinned
      x86-64 llama.cpp profile.
    - [x] Add an equivalently pinned, closed Ollama production package and
      installed evidence before claiming both supported providers.
  - [x] Pass the Phase 4 adversarial production-path Linux evidence while
    keeping private input disabled by default.
    - [x] Add a manually dispatched, pinned-input installed-service harness and
      content-free gateway probe.
    - [x] Record a successful llama.cpp run for merged commit `91830c3`,
      including filesystem denials, read-only package mounts, request-bound
      streaming cancellation, audit-capacity fail-closed behavior, orderly
      socket cleanup, stale-path refusal, and connect/header/completion
      cancellation races plus contained terminal-write refusal and recovery.
    - [x] Close the ADR-0018 terminal-write failure case for the installed
      llama.cpp path.
    - [x] Record equivalent installed Ollama isolation, inference,
      cancellation, race, audit, provider-loss and socket-lifecycle evidence in
      run `33866069049`.
    - [x] Build/test as non-root and assemble/inspect both provider packages in
      the pinned Arch x86-64 userspace in run `33867077966`.
- [x] Validate every model output and proposed tool intent against strict,
  code-owned schemas before it reaches the broker.
- [x] Produce controlled-protocol and real-model target-Linux evidence with
  external network access disabled.

Arbitrary shell remains a separately reviewed fallback capability in the target
security model; Phase 4 does not implement or expose it.

Exit: deterministic conformance tests pass for both providers, real local-model
operation is verified offline on target Linux, and no provider path bypasses the
broker, policy, approval, verification, or audit boundary. Unauthenticated
loopback endpoints receive no private input.

Exit satisfied: deterministic conformance covers both adapters; installed
Ubuntu x86-64 runs `33865392657` (llama.cpp) and `33866069049` (Ollama) exercise
real local models with provider external networking disabled; pinned Arch
x86-64 userspace run `33867077966` builds/tests the boundary and assembles and
inspects both package roots. The gateway remains the only private-input ingress,
model tool output remains a proposal, and no default build enables private
input. This is not Arch kernel, ArchISO, hardware, installer, or release proof.

The detailed requirement and evidence decision is recorded in
`docs/PHASE_4_EXIT_AUDIT.md`; installed evidence and its limitations are in
`docs/PHASE_4_INSTALLED_EVIDENCE.md`. Earlier `PHASE_4_*` documents remain
historical protected checkpoints and must be read in that chronology rather
than as current unresolved-gate statements.

## Phase 5: Planning and verification

Status: complete. ADR-0020 defines the closed orchestration state machine and
truthful terminal outcomes; `docs/PHASE_5_EXIT_AUDIT.md` records the evidence
and explicit recovery/cancellation limits.

- [x] Accept ADR-0020 for the closed plan schema, monotonic state machine,
  per-step authority checks, cancellation, partial/indeterminate outcomes,
  recovery and rollback limits, audit projection, and truthful summaries.
- [x] Add the closed bounded plan schema, code-derived capability projection,
  monotonic step lifecycle, terminal taxonomy, and truthful aggregate core.
- [x] Record plan acceptance, every step state, and the final typed aggregate in
  the engine's hash-chained, content-redacted audit log.
- [x] Translate only normalized, per-turn-eligible Phase 4 native-read intents
  into conservatively chained typed steps with runtime-derived identifiers.
- [x] Integrate intent, per-step policy, approval, execution, verification, and
  truthful-summary states over existing typed operations.
- [x] Handle dependency blocking, partial completion, rollback opportunities,
  cancellation, retry representation, and recovery limits.
- [x] Treat model, tool, and retrieved content as untrusted across adversarial
  end-to-end orchestration tests and record the exit audit.

Protected checkpoints: `docs/PHASE_5_PLAN_CORE.md` and
`docs/PHASE_5_TYPED_ORCHESTRATOR.md`, plus
`docs/PHASE_5_ORCHESTRATION_AUDIT.md` and
`docs/PHASE_5_INTENT_PLAN_BRIDGE.md`. Truthful reporting and the final decision
are recorded in `docs/PHASE_5_TRUTHFUL_REPORT.md` and
`docs/PHASE_5_EXIT_AUDIT.md`.

Exit satisfied: a plan reaches `completed` only when every required step has a
code-owned successful verification. Dispatch, exit status, provider output, or
tool return alone cannot produce success. Partial and indeterminate effects,
retry/rollback limits, and unsupported durable recovery remain explicit.

Exit: Blossom never reports success solely because a command was issued.

## Phase 6: Blossom Shell

Status: active. ADR-0021 is accepted. A Linux-only unprivileged session IPC
service is implemented behind an inactive production feature; no graphical
shell or installed-runtime evidence is currently claimed.

- [x] Review and accept the shell IPC and approval-surface ADR.
- [x] Freeze a closed, authenticated, versioned, size-bounded session IPC
  schema for only the existing fixed diagnostic slice. The closed Rust schema,
  adversarial decoder tests, real sender authentication, and private-bus
  transport test are implemented.
- [x] Implement the unprivileged session service without moving authorization,
  token custody, execution, verification, or audit authority into QML. The
  connection-bound pending state and fixed-diagnostic engine bridge are
  implemented with a bounded redacted projection of authoritative audit stages.
  A Linux-only session D-Bus service adapter now exists behind an inactive
  production feature. Installed evidence is tracked separately below.
- [ ] Prove hostile same-user caller, replay, mutation, expiry, disconnect,
  restart, cancellation, schema, and service-loss behavior fails closed. Client
  unique-name loss now cancels its pending approval through the audited engine
  path, and loss of the owner-monitor stream terminates the service path; the
  private-bus suite also proves cross-peer preview theft, preview mutation,
  approval replay, exact cancellation, and cancellation replay against the real
  engine with an execution counter. Restart and installed service-loss cases
  keep this item open.
- [ ] Establish the pinned Arch + Hyprland + Quickshell development and package
  boundary.
- [ ] Build exact approval preview and readable correlated activity UI for the
  fixed diagnostic.
- [ ] Add launcher, notifications, system status, and agent surfaces only as
  separately reviewed increments after the first slice preserves the boundary.
- [ ] Produce installed target evidence and an independent Phase 6 exit audit.

The active design baseline and exit evidence are tracked in
`docs/PHASE_6_BASELINE.md`. Phase 6 adds no capability merely to make a UI
demonstration work.

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
