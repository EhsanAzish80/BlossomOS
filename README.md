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

Phases 0 through 10 are complete at their separately reviewed boundaries.
Phase 11 is active with one exact physical-device qualification boundary and a
fail-closed physical installer. A first installation and console boot have been
observed, but desktop startup, the hardware matrix, rollback, and recovery are
not yet qualified or supported.
Phase 10 produced a verified x86-64 UEFI VM beta-candidate evidence bundle; it
did not publish a release or establish physical-device support.

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

## What it can do today

Blossom's agent currently performs exactly one action end to end: **creating a
new file inside a workspace folder**. The model proposes a file name and its
content; you see the exact name and content, approve once, and Blossom writes
the file, verifies it, and records the outcome.

That loop passes automated tests with a stand-in (fixture) model provider. It
also passed 20 consecutive real-model runs from a freshly built ARM64 image:
all 20 intended file creations were verified, while the 80 adversarial cases
produced no effects and started no command executor. See the
[ARM64 image-gate evidence](docs/ARM64_AGENT_IMAGE_GATE_EVIDENCE.md) for the
exact boundary and limitations. Everything else the agent might do in future
is out of scope until it has the same end-to-end evidence.

## How an action happens

1. **The model can only propose.** A local model returns at most one proposal
   in a fixed, closed format: a file name and its content. It has no shell, no
   free-form commands, and no way to approve or run anything itself.
2. **Trusted code checks the proposal.** Names are limited to one lowercase
   ASCII path component (at most 64 bytes). Content is limited to 4 KiB, with
   NUL bytes, terminal escapes, and invisible or direction-changing Unicode
   characters rejected. The destination is resolved inside the workspace
   folder; traversal, absolute paths, symlink tricks, and files swapped after
   checking are rejected. Existing files are never overwritten.
3. **You see the exact action.** The approval preview shows the exact name and
   content that will be written.
4. **Approval covers only that action.** An approval is bound to that exact
   preview, can be used once, and expires after 30 seconds. Changing the
   request after approval invalidates it.
5. **It runs without root and is verified.** The file is created with mode
   `0600` through the already-open, checked workspace directory, then verified
   by digest.
6. **It is logged without the content.** The activity record keeps the origin,
   outcome, content digest, and length, never the file content itself.

The model runs as an isolated system service with no network access, reached
only through a local gateway. Model output is always treated as untrusted
input, and model-proposed actions can never be allowed automatically.

## What has been tested, and what that does not prove

- Adversarial tests cover path traversal, symlinks, file swaps between checking
  and writing, prompt injection in the request, replayed or changed approvals,
  and approval-capacity exhaustion.
- The request parser and resolver were fuzzed with about 33 million malformed
  inputs on macOS and Linux with no crashes. That shows malformed input does
  not crash this boundary; it does **not** show that the system is secure.
- There has been **no independent security review**.
- Self-approval without authentication is proven to fail closed; the
  interactive password path is pending manual verification. The remaining
  check is maintained in
  [trusted approval manual evidence](docs/TRUSTED_APPROVAL_MANUAL_EVIDENCE.md).
  Overlay phishing and same-user authentication-agent substitution remain
  documented risks; see
  [ADR-0034](docs/decisions/0034-trusted-approval-authentication.md).

Security researchers are welcome to try to break these boundaries. Please
report findings through the [security policy](SECURITY.md).

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
defaults. The reviewed Phase 9 path and completed Phase 10 candidate remain
intentionally limited to repeatable x86-64 UEFI VM evidence.

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
