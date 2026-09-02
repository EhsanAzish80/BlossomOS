# Blossom OS

Blossom OS is an open-source, local-first, agent-native Linux desktop project.
Its target platform is Arch Linux with Hyprland and a custom Quickshell shell,
backed by a replaceable local AI runtime, capability broker, policy engine,
sandboxed executor, minimal privileged helper, and structured audit system.

That is the target architecture, not the current implementation.

## Project status

Blossom OS is pre-alpha research software. It is not ready for installation as a
trusted daily operating system and has no supported release.

### Implemented today

The repository currently contains the preserved original prototype:

- ArchISO and experimental Alpine build/setup scripts.
- XFCE and Picom configuration.
- A Python command-line assistant with rule-based responses and system inspection.
- VM, installer, theme, autologin, and bootable-media helper scripts.
- Early prototype documentation.

The untouched initial state is preserved by the Git tag
`prototype-pre-agent-architecture`.

The Python assistant does **not** load or run a real LLM. The legacy scripts have
not been validated as a secure or production-ready distribution and include
insecure development defaults. See `SECURITY_MODEL.md` and
`docs/PHASE_0_BASELINE.md` before running them.

Phase 1 also implements a separate Rust security vertical slice:

- One typed `system.uname` request mapped only to `/usr/bin/uname -s`.
- A deny-by-default capability policy and exact, once-only terminal approval.
- Approval binding, expiry, cancellation, and replay protection.
- A Linux Bubblewrap adapter with a read-only system view, isolated network,
  cleared environment, dropped capabilities, timeout, and output limits.
- Result verification and hash-chained, content-redacted audit activity.
- Non-interactive denial and no unsandboxed fallback.

This slice is a tested foundation, not a general command runner or finished OS.

Phase 2 currently adds two native read capabilities: `system.read:os.identity`
and `system.read:uptime`. They parse fixed Linux sources without launching a
process, expose narrow typed results, and record verified provenance without
copying identity values or uptime into the audit log. Later Phase 2 capabilities
are not implemented.

### Planned, not implemented

The following are architectural goals only:

- Hyprland integration and a Quickshell-based Blossom Shell.
- A provider-neutral local AI runtime with replaceable model backends.
- A typed Blossom Bus and structured desktop/system context.
- A minimal typed privileged helper with no generic root shell.
- Persistent, user-manageable local audit storage.
- Agent planning, execution verification, memory, packaging, and safe updates.

No security property described in the target documents should be treated as a
claim about the current prototype.

## Architectural contract

- `VISION.md` defines what Blossom is and is not.
- `ARCHITECTURE.md` defines target components and trust boundaries.
- `SECURITY_MODEL.md` defines required security properties and tests.
- `ROADMAP.md` defines phased delivery and exit gates.
- `CONTRIBUTING.md` defines contribution and review expectations.
- `SECURITY.md` describes the current security and reporting status.
- `docs/DEPENDENCY_POLICY.md` defines dependency acceptance and update rules.
- `docs/BRANCH_RELEASE_POLICY.md` defines branch, review, versioning, and signing.
- `docs/decisions/` records accepted and proposed architecture decisions.

Material architecture changes require an accepted ADR and matching updates to
the relevant contract documents.

## Repository layout today

```text
ai-core/               rule-based Python prototype
apps/blossom-cli/      Phase 1 interactive approval and activity client
build/                 prototype ArchISO/Alpine build scripts and helpers
config/                prototype XFCE/Picom configuration
core/blossom-core/     typed policy, approval, executor, verification, and audit core
docs/                  prototype docs, Phase 0 evidence, and ADRs
scripts/               prototype installation and utility scripts
```

The target layout in `ARCHITECTURE.md` has not been created. The prototype has
not been moved or restructured.

## Development

Phases 0 and 1 are complete. Phase 2 expands the capability and sandbox
foundation through typed tools and containment tests; it must not integrate an
LLM or restructure the preserved prototype merely for appearance.

Prototype commands in older documentation are historical development material,
not supported installation instructions.

## Contributing and security

Read `CONTRIBUTING.md` before proposing changes. Do not report exploitable
vulnerabilities in public issues; follow `SECURITY.md` for the current reporting
status.

## License

Blossom OS is licensed under the Apache License, Version 2.0. See `LICENSE` and
accepted ADR-0001.
