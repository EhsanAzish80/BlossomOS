# ADR-0004: Phase 2 capability taxonomy and expansion rules

- Status: Accepted
- Date: 2026-09-02
- Owners: Project maintainers

## Context

Phase 1 proves one deterministic path from a typed request through policy,
once-only approval, Bubblewrap execution, verification, and redacted audit
activity. Phase 2 must add useful operations without letting individually narrow
tools compose into a generic command runner, broad surveillance interface, or
ambient filesystem authority.

Read access is not inherently harmless. Process metadata, filenames, service
state, storage layout, and host identity can reveal private activity or improve
fingerprinting. Write access adds path traversal, symlink, race, and destructive
side effects. A caller-controlled executable, argument vector, sandbox profile,
mount, namespace, or environment would bypass the capability model entirely.

The expansion rules therefore need to be fixed before the first Phase 2 tool is
implemented.

## Decision

### Typed authority

Every operation is a dedicated, closed request type with a dedicated verifier.
The authoritative capability is a Rust type, not a caller-provided capability
string. Human-readable names are stable display and policy identifiers derived
from that type.

A capability consists of:

1. a domain, such as `system`, `process`, `files`, or `services`;
2. a verb, initially `read` or `write`;
3. a resource class, such as `os.identity`, `list`, or `content`; and
4. a typed resource scope where the class is not a singleton.

Scopes must identify the actual resource. Examples include an exact normalized
file path, a workspace root plus relative path, an own-user process class, or an
exact service unit. Wildcards, arbitrary strings interpreted as patterns, and
unbounded `all` scopes are prohibited unless a later ADR includes a threat and
composition review.

Requests may narrow a statically declared tool scope but cannot expand it. The
broker derives the required capability from the parsed request and rejects any
caller-supplied capability assertion.

### Tool and execution ownership

Each registered tool owns, in code:

- its request schema and size limits;
- required capability and scope derivation;
- privacy classification and default policy;
- normalized operation or native API implementation;
- sandbox or confinement profile, when needed;
- timeout, output, and resource limits;
- result schema, redaction rules, and verifier; and
- adversarial and target-Linux tests.

Callers, future model providers, and request payloads cannot select executables,
arguments, environment variables, working directories, mounts, namespaces,
Bubblewrap flags, seccomp rules, or network access. There is no generic
`execute`, `read path`, `run diagnostic`, or equivalent escape hatch hidden
behind a broad request type.

Native Rust/Linux APIs and bounded `/proc` parsing are preferred over launching
commands. If a command-backed implementation is ever necessary, the absolute
program, complete argument vector, and sandbox profile must be fixed in code and
reviewed as a separate security boundary. It must not invoke a shell.

### Expansion order

Capabilities are introduced one checkpoint at a time in this order:

1. `system.read:os.identity`
2. `system.read:uptime`
3. `system.read:memory.summary`
4. `system.read:storage.summary`
5. `process.read:self`
6. `process.read:list` with explicit once-only approval
7. `files.read:content` for one user-selected exact file
8. `files.write:content` for one exact path beneath an approved workspace root
9. `services.read:status` for one exact service unit

The existing Phase 1 `system.read:kernel.identity` diagnostic remains fixed and
does not become a generic system command facility.

Later entries may begin only after earlier entries meet their exit evidence.
This order is not authorization to implement the full list in one change.

### Policy and privacy

Default deny remains universal. `process.read:list` requires explicit once-only
approval. Single-file reads, workspace writes, and service status reads must
receive an explicit policy decision based on their exact scope; selection of a
resource does not silently create a persistent grant. No Phase 2 operation may
add permanent approval.

Audit records store the minimum metadata needed to correlate and verify an
operation. File contents, process command lines, environment values, filenames,
and other sensitive results are not logged by default. Human activity views use
redacted summaries and audit identifiers.

Privacy and composition are reviewed independently of mutability. A new tool
must document what it reveals, how its output could combine with existing tools,
and why the combined set still cannot express generic execution or ambient
surveillance.

### `/proc` constraints

`/proc` readers may open only the files required by their dedicated tool and
must use bounded reads and strict parsers. Unknown fields and malformed,
oversized, disappearing, or permission-denied entries fail closed or produce a
typed partial result where the schema explicitly permits it.

The initial process tools must not expose other processes' environment,
credentials, open file paths, sockets, memory, stack, or unrestricted command
lines. PID reuse and disappearing processes must be tested. `process.read:self`
and `process.read:list` are different capabilities and implementations.

### Filesystem constraints

File tools are blocked until exact-path containment is proven with adversarial
tests. Lexical checks alone are insufficient.

Single-file reads must bind approval to the exact selected resource and reject
relative paths, `..`, NUL/control characters, non-regular files, and symlinked
path components. The implementation must use descriptor-relative Linux APIs or
equivalent kernel-enforced resolution and must revalidate resource identity to
address selection-to-open races.

Workspace writes must additionally bind authority to an approved workspace root
and one exact relative destination. They must prevent escape through symlinks,
mount changes, replacement races, and special files. Atomic replace, creation,
overwrite, permissions, durability, backup, and rollback behavior require an
explicit design before the first write is enabled.

Writable filesystem access may not be introduced in the same checkpoint that
first establishes read containment.

### Per-capability exit evidence

Every new tool requires all of the following before merge:

- typed request and typed result with unknown-field rejection and size bounds;
- statically derived capability and exact resource scope;
- deny-by-default policy plus an exact human-readable approval preview where
  policy returns `ask`;
- code-owned implementation and confinement profile;
- deterministic verification and complete redacted audit transitions;
- success, denial, cancellation, expiry, replay, malformed-input, unavailable,
  timeout, and output-bound tests where applicable;
- adversarial tests for the resource class, including `/proc`, traversal,
  symlink, and TOCTOU behavior where applicable;
- privacy and cross-tool composition review; and
- a target-Linux integration test proving the real containment boundary.

No LLM, Bash interface, sudo path, privileged helper, IPC daemon, Hyprland, or
Quickshell implementation is part of the Phase 2 capability-foundation work.

## Alternatives considered

- One generic command tool with an allowlist: rejected because argument,
  environment, executable replacement, and tool composition would recreate a
  shell-shaped authority boundary.
- A generic filesystem API with runtime path policy: rejected for the initial
  expansion because read, selection, workspace write, creation, and replacement
  have different privacy and race properties.
- Treat all read-only operations as automatically allowed: rejected because
  system, process, filename, and service metadata can be sensitive.
- Implement the whole capability list before review: rejected because failures
  and composition risk would be harder to isolate and roll back.
- Let callers submit Bubblewrap profiles: rejected because sandbox policy is
  executable authority and must remain code-owned.

## Security and privacy consequences

This decision keeps authority finite and reviewable as Phase 2 grows. It adds
friction: similar-looking operations may require separate types, implementations,
and tests. That duplication is intentional where security or privacy properties
differ.

Native APIs reduce command-injection surface but do not automatically provide
containment. Each tool still needs least-access design, output minimization, and
target-Linux evidence. Bubblewrap remains selected only for the Phase 1 profile
until a later tool explicitly adopts or supersedes that profile through review.

## Operational consequences

The registry, policy engine, CLI preview, verifier, audit renderer, and tests will
grow incrementally. Phase 2 changes should remain small enough for one capability
and its containment evidence to be reviewed together. Platform-independent
parsers may be tested on macOS, while Linux API and containment claims require
Linux CI and later target-Arch validation.

## Migration and rollback

The first Phase 2 change extends the existing typed enums and registry without
changing the Phase 1 request or Bubblewrap profile. Each later capability can be
removed independently by deleting its request variant, policy mapping,
implementation, renderer, and tests. Stored or persistent permissions are not
introduced, so rollback does not require grant migration.

Changing this taxonomy or relaxing an expansion rule requires an ADR that
supersedes this one.

## Validation

- Contract review against `VISION.md`, `ARCHITECTURE.md`, `SECURITY_MODEL.md`,
  `ROADMAP.md`, and accepted Phase 1 ADRs.
- Repository checks ensure the ADR and roadmap remain tracked and reviewable.
- Each implementation checkpoint supplies the per-capability exit evidence above.
