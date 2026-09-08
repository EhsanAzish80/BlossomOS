# Blossom OS

Blossom OS is an open-source, local-first, agent-native Linux desktop project.
It explores how a desktop agent can be useful without receiving ambient
authority over the computer.

The target system combines Arch Linux, Hyprland, a Blossom-owned Quickshell
interface, replaceable local model providers, typed capabilities, explicit
approval, sandboxed execution, minimal privilege, verification, and
content-minimized audit.

> [!WARNING]
> Blossom OS is pre-alpha research software. It is not a supported operating
> system release and is not ready to protect a daily-use machine.

## Current status

Phases 0 through 9 are complete at their separately reviewed boundaries.
Phase 10—public-beta hardening—is next.

Implemented and tested foundations include:

- deny-by-default typed capabilities and exact once-only approvals;
- bounded native system, process, file, storage, and service operations;
- a minimal independently authorized privileged helper;
- replaceable local Ollama and llama.cpp provider boundaries;
- authenticated, bounded, provider-neutral model gateway protocols;
- closed multi-step planning with verification-derived outcomes;
- a narrow Qt/QML approval and authoritative activity surface;
- fixed battery and coarse network-connectivity context projections; and
- disabled-by-default encrypted durable notes with explicit lifecycle controls
  and bounded data-only recall; and
- closed Arch packages, a Blossom-owned UEFI VM image, and signed offline A/B
  update, rollback, confirmation, and recovery foundations.

Every completed phase has deterministic tests and an exit record. Selected
boundaries also have installed Linux evidence. These results establish only the
documented slices. The Phase 9 installer is an evidence-only VM path; these
results are not public-release, broad-hardware, physical-device, or daily-use
claims.

See the [roadmap](ROADMAP.md) for completion gates and the
[documentation hub](docs/README.md) for evidence and architecture decisions.

## Security model

Blossom treats model output, prompts, files, tool output, remembered data, and
same-user processes as untrusted input—not authorization.

Core rules:

- capabilities are narrow, typed, and derived by trusted code;
- privileged or sensitive effects require an exact user-visible approval;
- approval is once-only, request-bound, expiring, and non-transferable;
- execution uses operation-specific containment with no generic shell fallback;
- success is reported only after operation-specific verification;
- audit records outcomes and provenance without copying private content; and
- local-first means no hidden network dependency, telemetry, or cloud fallback.

The authoritative requirements and current limitations are in the
[security model](SECURITY_MODEL.md). Report vulnerabilities through the
[security policy](SECURITY.md), not a public issue.

## Architecture

The target architecture separates presentation, model inference, policy,
execution, privilege, verification, memory, and audit authority. A model may
propose a closed intent; it cannot approve or execute that intent by itself.

```text
User and Blossom Shell
          |
          v
Typed request -> policy -> exact approval -> contained operation
                                             |
                                             v
                                  verification -> audit

Local model provider -> untrusted intent proposal only
```

Read [ARCHITECTURE.md](ARCHITECTURE.md) for component and trust boundaries and
[VISION.md](VISION.md) for the product principles.

## Repository structure

```text
apps/                 user-facing Rust clients
core/blossom-core/    typed policy, approval, execution, verification, and memory core
system/               isolated service and provider boundaries
docs/                 phase evidence, policies, and architecture decisions
.github/workflows/    protected checks and installed-evidence workflows
ai-core/              preserved historical Python prototype
build/, config/, scripts/
                      preserved experimental distribution prototype
```

The historical prototype is preserved under the
`prototype-pre-agent-architecture` tag. Its scripts and setup guides are not
supported installation instructions and may contain insecure development
defaults. The reviewed Phase 9 path is intentionally limited to repeatable
x86-64 UEFI VM evidence while Phase 10 owns public-release hardening.

## Development

The workspace requires the Rust toolchain pinned in
[rust-toolchain.toml](rust-toolchain.toml). Some integration tests additionally
require Linux, Bubblewrap, D-Bus, Qt 6, and related system packages.

Run the portable local gates:

```bash
python3 scripts/ci/check_repository.py
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
```

Loopback-based provider tests may require a normal host environment if a
development sandbox prohibits local sockets. Installed and architecture-
specific evidence is produced only by the corresponding reviewed workflows.

Before changing a trust boundary, read:

1. [CONTRIBUTING.md](CONTRIBUTING.md)
2. [SECURITY_MODEL.md](SECURITY_MODEL.md)
3. [ROADMAP.md](ROADMAP.md)
4. [Architecture decisions](docs/decisions/README.md)

Material architecture changes require a focused ADR and matching tests,
evidence, documentation, and rollback analysis.

## Project documents

- [Documentation hub](docs/README.md)
- [Vision](VISION.md)
- [Architecture](ARCHITECTURE.md)
- [Security model](SECURITY_MODEL.md)
- [Roadmap](ROADMAP.md)
- [Contributing](CONTRIBUTING.md)
- [Security reporting](SECURITY.md)
- [Dependency policy](docs/DEPENDENCY_POLICY.md)
- [Branch and release policy](docs/BRANCH_RELEASE_POLICY.md)

## License

Blossom OS is licensed under the [Apache License 2.0](LICENSE).
