# Phase 0 Baseline

Last reviewed: 2026-09-02

This document records repository evidence for the preservation-and-baseline
phase. It is not a claim that the target agent-native system exists.

## Completion checklist

### Prototype preservation

- Status: complete.
- Evidence: root commit `bc2bb05` preserves the original prototype unchanged.
- Evidence: annotated tag `prototype-pre-agent-architecture` resolves to that
  commit.
- The prototype remains in its original directories; it has not been moved to
  `legacy/` or restructured.

### Current repository state

- Status: complete for initial publication.
- The GitHub repository is public and uses `main` as its default branch.
- The reviewed initial history and preservation tag are published to `origin`.
- Phase 0 documentation follows the preservation commit as separate commits.

### License

- Status: complete.
- ADR-0001 is accepted and selects Apache-2.0.
- `LICENSE` contains the canonical Apache License 2.0 text.
- README and contributor guidance identify the same license.
- Dependency and package-metadata compatibility remains a future per-dependency
  review because no package manifest or release metadata exists yet.

### Authoritative project documents

- Status: complete for Phase 0.
- `VISION.md`, root `ARCHITECTURE.md`, `SECURITY_MODEL.md`, `ROADMAP.md`,
  `CONTRIBUTING.md`, and `SECURITY.md` form the current project contract.
- `docs/ARCHITECTURE.md` and `docs/QUICK_START.md` belong to the preserved
  prototype and are not target-architecture or supported-installation guidance.
- Architecture changes require an accepted ADR.

### Implemented versus aspirational

- Status: documented.
- Implemented: prototype ISO/setup scripts, XFCE/Picom configuration, VM helpers,
  and a rule-based Python CLI that gathers limited system information and returns
  canned guidance.
- Not implemented: a real LLM runtime, Hyprland/Quickshell shell, Blossom Bus,
  capability broker, policy engine, approval protocol, sandboxed executor,
  privileged helper, structured audit service, agent planner, or verification
  state machine.

### Contributor rules

- Status: documented but not enforced by repository settings or CI.
- `CONTRIBUTING.md` defines review, architecture, test, documentation, and Git
  expectations.
- External changes must not be treated as authorization for system operations.

### Vulnerability reporting

- Status: incomplete; repository-owner action required.
- `SECURITY.md` keeps the project explicitly pre-alpha and disclaims support.
- GitHub Private Vulnerability Reporting was verified disabled on 2026-09-02.
- Owner action: enable Private Vulnerability Reporting in GitHub repository
  settings, verify the private reporting form, and add its active link to
  `SECURITY.md`.
- GitHub secret scanning and push protection were verified enabled. Non-provider
  pattern scanning, validity checks, and Dependabot security updates were
  verified disabled. These are repository settings, not implemented controls in
  Blossom itself.

### Branch and release policy

- Status: incomplete.
- `main` is the default branch and contributor guidance says not to rewrite shared
  history, but GitHub reported no branch protection on 2026-09-02.
- No supported releases exist. Signing, versioning, support, and release-channel
  policy have not been defined.
- Owner/project action: approve a branch/review policy, configure the matching
  GitHub ruleset or branch protection, and document the initial release policy.

### CI, lint, tests, and security scanning

- Status: incomplete.
- No `.github/workflows` directory or GitHub Actions workflows exist.
- No repository-wide formatter, linter, test runner, dependency manifest,
  dependency update policy, or code security-scanning configuration was found.
- Existing shell and Python files are prototype material; no automated result
  currently qualifies them as tested or secure.
- Project action: select Phase 1 implementation language/tooling by ADR, then
  define the minimum Phase 0 quality gates that must run before Phase 1 merges.

## Prototype safety and repository-hygiene findings

The scan covered tracked files and all current Git objects for obvious credential
terms and large objects. It is a focused baseline review, not a complete secret or
security audit.

### Credentials and unsafe defaults

- No private keys, API tokens, model weights, or large binary files were found by
  the focused scan.
- `scripts/install.sh`, `scripts/alpine-setup.sh`, and
  `scripts/one-line-setup.sh` set the predictable password `blossom`.
- `scripts/auto-setup.sh` contains placeholder `mypassword` input and advertises
  the same predictable account password.
- Alpine setup scripts add `%wheel ALL=(ALL) NOPASSWD: ALL` to sudoers.
- Autologin and root-oriented setup behavior exists throughout the prototype.

These are preserved historical development defaults, not production defaults.
Do not run or ship the legacy installers on a trusted system.

### Generated and machine-specific material

- `build/out/customize-alpine.sh` is a generated customization script committed
  in the original prototype.
- `docs/QUICK_START.md` contains a developer-specific absolute path under
  `/Users/ehsanazish/...`.
- Multiple scripts assume a fixed `/home/blossom` account and paths.
- No file larger than 1 MiB was found. No model files, ISO images, archives, or
  compiled binaries are tracked.

These files remain untouched to preserve history. A later reviewed migration may
quarantine, replace, or remove them; Phase 0 does not restructure the prototype.

### Destructive or high-impact prototype behavior

- ISO/build scripts remove build directories and may install host packages.
- Bootable-media tooling can write to a user-selected block device.
- Setup scripts alter users, passwords, sudoers, services, repositories, and
  desktop configuration, and some reboot automatically.

The current prototype is not a safe installer or supported build. Documentation
must not present these commands as production-ready.

## Phase 0 exit decision

Phase 0 remains **in progress**. Preservation, publication, licensing, contract
documents, implementation inventory, and the focused hygiene audit are complete.
The exit gate is not satisfied until:

1. GitHub Private Vulnerability Reporting is enabled and linked.
2. Branch/review and initial release policies are approved and enforced where
   applicable.
3. Baseline formatting, linting, tests, CI, dependency policy, and security
   scanning requirements are defined and present before Phase 1 merges.

Phase 1 implementation must not begin before this status is deliberately updated.
