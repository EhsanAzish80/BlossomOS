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

Phase 2 currently adds eight native read capabilities:
`system.read:os.identity`, `system.read:uptime`, and
`system.read:memory.summary`, root-scoped `system.read:storage.summary`, and
`process.read:self`, plus approval-gated `process.read:list`.
They use fixed native Linux/POSIX sources without launching a process, expose
narrow typed results, and record verified provenance without copying identity,
uptime, memory, storage, or process identifiers into the audit log.
`process.read:self` is restricted to Blossom's own minimal native identity;
`process.read:list` returns at most 256 same-effective-user PIDs, short kernel
names, and coarse states after explicit once-only approval. It never reads
command lines, environments, open files, sockets, or process memory.

The seventh capability, `files.read:content`, requires an absolute user-selected
path and once-only approval. On Linux it uses `openat2` to reject symlinked path
components, retains the selected descriptor across approval, revalidates file
identity, and reads at most 64 KiB of UTF-8 text. It provides no relative paths,
directory listing, globbing, binary-file transport, or write access.

Phase 2 also implements one narrowly scoped write capability:
`files.write:create`. After explicit once-only approval, it can atomically
create one previously absent UTF-8 file beneath a selected workspace using
retained directory descriptors, fixed `0600` permissions, verified temporary
content in an unnamed `O_TMPFILE` inode, atomic no-replace publication with
`linkat(AT_EMPTY_PATH)`, and file/directory durability sync. It cannot
overwrite, append, delete, choose permissions, follow symlinks, or cross a
nested mount.

The eighth read capability, `services.read:status`, observes one exact loaded
system-manager `.service` unit after explicit once-only approval. It uses fixed
native systemd D-Bus calls for only `GetUnit`, `Id`, `LoadState`, `ActiveState`,
and `SubState`, with a fixed local bus address and bounded deadlines. It cannot
list or load units, request all properties, mutate systemd, invoke `systemctl`,
or send a caller-selected D-Bus message. Service names and states are omitted
from persistent audit detail.

Phase 3 implements one narrowly typed privileged operation:
`services.restart:bluetooth.service`. An interactive CLI displays the exact
fixed operation and grants approval once only. A system D-Bus service captures
the authenticated sender and UID, independently requests the fixed polkit
action, persists replay-safe journal state, calls only
`TryRestartUnit("bluetooth.service", "replace")`, verifies the changed active
invocation, and writes a bounded, synced, hash-chained audit trail. The helper
cannot accept a shell command, executable, argument vector, caller-selected
unit, D-Bus address, method, object path, polkit action, or job mode.

The Phase 3 package boundary is validated on controlled Linux CI services but
is not installed by this repository, and real target-Arch or Bluetooth-hardware
behavior has not yet been claimed. See `docs/PHASE_3_BASELINE.md`.

### Planned, not implemented

The following are architectural goals only:

- Hyprland integration and a Quickshell-based Blossom Shell.
- An authenticated, packaged, user-ready local AI runtime and model lifecycle.
- A typed Blossom Bus and structured desktop/system context.
- A complete user-facing, persistent and manageable audit service beyond the
  current security-core and privileged-helper audit backends.
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

Phases 0 through 3 are complete. Their exit evidence is recorded in
`docs/PHASE_0_BASELINE.md`, `docs/PHASE_1_SECURITY_CORE.md`,
`docs/PHASE_2_BASELINE.md`, and `docs/PHASE_3_BASELINE.md`. Phase 4 is active
under accepted ADR-0011, beginning with provider-neutral types and synthetic
conformance fixtures. The closed core contract is implemented and documented in
`docs/PHASE_4_CORE_CONTRACT.md`. A fixed-loopback, synthetic-only Ollama
development adapter is documented in `docs/PHASE_4_OLLAMA_ADAPTER.md`, and an
equally constrained llama.cpp adapter is documented in
`docs/PHASE_4_LLAMA_CPP_ADAPTER.md`. These two adapters demonstrate a replaceable
provider boundary against controlled protocol fixtures; they are not an
authenticated or production model runtime. No private input, model download,
provider lifecycle, cross-provider real-model proof, or offline real-model
evidence exists, pending complete implementation and validation of the accepted
provider-identity boundary.

ADR-0012 now defines that provider-identity boundary. Its first implementation
checkpoint adds only closed, synthetic gateway framing, strict request/event
validation, cancellation binding, and Linux Unix-socket peer-credential
primitives. It does not create or install the gateway service, authenticate a
production provider, or permit private input. See
`docs/PHASE_4_GATEWAY_PROTOCOL.md`.

A second synthetic-only checkpoint connects a bounded client to a one-request
fixture gateway over an ephemeral Unix socket. Target-Linux tests run the
gateway in a separate process and prove that a peer-identity mismatch closes the
connection before any request byte is written. This remains test infrastructure,
not the packaged production gateway. See `docs/PHASE_4_GATEWAY_FIXTURE.md`.

Prototype commands in older documentation are historical development material,
not supported installation instructions.

## Contributing and security

Read `CONTRIBUTING.md` before proposing changes. Do not report exploitable
vulnerabilities in public issues; follow `SECURITY.md` for the current reporting
status.

## License

Blossom OS is licensed under the Apache License, Version 2.0. See `LICENSE` and
accepted ADR-0001.
