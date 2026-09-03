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

- Status: historical Phase 0 inventory; superseded for current implementation
  status by the root README and later phase baselines.
- At Phase 0, only the prototype ISO/setup scripts, XFCE/Picom configuration,
  VM helpers, and rule-based Python CLI existed.
- Phases 1 through 3 have since implemented the separate trusted Rust security
  core, closed capabilities, sandboxed fixed diagnostic, verification/audit,
  and one narrowly typed privileged helper operation. A real LLM runtime,
  Hyprland/Quickshell shell, Blossom Bus, and agent planner remain unimplemented.

### Contributor rules

- Status: documented and enforced by repository policy, CI, and protected
  `main` settings as of 2026-09-03.
- `CONTRIBUTING.md` defines review, architecture, test, documentation, and Git
  expectations.
- External changes must not be treated as authorization for system operations.

### Vulnerability reporting

- Status: complete.
- `SECURITY.md` keeps the project explicitly pre-alpha and disclaims support.
- GitHub Private Vulnerability Reporting and its private reporting form were
  verified enabled on 2026-09-02; `SECURITY.md` links to that form.
- GitHub secret scanning and push protection were verified enabled. Non-provider
  pattern scanning, validity checks, and Dependabot security updates were
  verified disabled. These are repository settings, not implemented controls in
  Blossom itself.

### Branch and release policy

- Status: complete for Phase 0.
- `main` is the default branch. Branch protection was verified enabled on
  2026-09-02 and applies to administrators.
- Changes require a pull request, current required checks, and resolved review
  conversations. Force-pushes and branch deletion are disabled.
- `docs/BRANCH_RELEASE_POLICY.md` defines the initial branch, review, versioning,
  release, checksum, and signing policy. No supported releases exist.
- Required checks are `Repository checks`, `Analyze Python`, `Gitleaks`, and
  `Dependency review`.

### CI, lint, tests, and security scanning

- Status: complete for the Phase 0 baseline.
- `Quality` runs dependency-free repository checks, characterization tests, and
  ShellCheck at error severity.
- `CodeQL` analyzes the prototype Python on pushes, pull requests, and weekly.
- `Secret scan` runs Gitleaks against full history.
- Dependabot checks immutable-pinned GitHub Actions weekly.
- `docs/DEPENDENCY_POLICY.md` defines dependency review and lockfile rules.
- ADR-0002 selects stable Rust for the Phase 1 security core; Rust format,
  Clippy, and test gates become required when its workspace is introduced.
- Local evidence on 2026-09-02: repository checks passed, three prototype smoke
  tests passed, all tracked shell scripts passed syntax parsing and ShellCheck at
  error severity, and Actionlint passed.
- Default-branch evidence on commit `42aa8d0`: Quality, CodeQL, and Secret scan
  completed successfully.
- Later phases added strict Rust formatting, Clippy, target-Linux tests,
  privileged packaging validation, and dependency review. The dependency graph
  and required `Dependency review` check were verified enabled on 2026-09-02.

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

Phase 0 is **complete** as of 2026-09-02. Preservation, publication, licensing,
contract documents, implementation inventory, hygiene audit, private reporting,
branch/release policy, protected review flow, and repository quality/security
gates have direct evidence.

Phase 1 may begin on a separate branch. It remains limited to the deterministic
broker -> policy -> approval -> sandboxed execution -> verification -> audit
vertical slice. No LLM, privileged helper, shell migration, or prototype
restructuring is authorized by Phase 0 completion.
