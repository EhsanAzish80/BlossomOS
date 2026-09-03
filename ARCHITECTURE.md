# Blossom OS Target Architecture

Status: authoritative target contract. The older `docs/ARCHITECTURE.md`
describes the preserved prototype and is not the target design.

## System shape

```text
Human
  |                         Local model provider
  v                                |
Blossom Shell <-> Blossom Bus <-> Agent Runtime / Planner
                         |          |
                         |     structured tool request
                         v          v
                   Capability Broker
                         |
                    Policy Engine
                    /          \
               allow/deny      ask
                  |             |
                  |       Shell approval UI
                  |             |
                  +------v------+
                         |
                  Sandboxed Executor
                         |
              minimal privileged helper
                  (only when required)
                         |
                    Linux / systemd

Every decision and result -> append-only structured audit log
```

## Component boundaries

### Blossom Shell

A Quickshell/QML process running on Hyprland. It owns user-facing desktop
surfaces: launcher, agent panel, approval prompts, notifications, system status,
and activity history. It never makes authorization decisions.

### Blossom Bus

A typed local IPC boundary connecting shell and services. Messages are
versioned, authenticated to the local session, size-bounded, and schema-checked.
Transport will be selected by ADR; D-Bus is the initial candidate.

### Agent Runtime

Maintains conversation state, asks a replaceable model provider for structured
plans, validates returned schemas, and invokes only registered tools. A planner
cannot directly execute a process or elevate privileges.

### Model providers

Adapters expose a common interface for llama.cpp, Ollama, and future local or
explicitly enabled external providers. Provider selection must not alter policy
or executor behavior.

ADR-0011 treats providers and their output as untrusted. The model sees an empty
tool catalogue by default and only a minimal code-owned intent allowlist for an
eligible turn. Returned calls are proposals, never executable requests. Fixed
loopback HTTP is development transport only: real adapters are restricted to
synthetic prompts until a reviewed endpoint-identity boundary binds the actual
connection to the expected provider service. Mixed text-and-action completions
fail closed.

### Capability Broker and Policy Engine

The broker is the only route from a tool request to execution. Tools declare
capabilities and scopes. The policy engine returns `allow`, `deny`, or `ask` plus
the reason and effective scope. The model cannot grant or persist permissions.

### Sandboxed Executor

Runs an approved action as an unprivileged user with a filtered environment,
bounded working directory, timeout, output limit, resource limits, and explicit
filesystem/network policy. It captures stdout, stderr, exit status, and
termination reason.

### Privileged Helper

A small, independently reviewable service that accepts a closed set of typed
operations. It does not expose a generic root shell and revalidates authorization
at the privilege boundary. Polkit is the initial authorization candidate.

### Audit Service

Records requests, plans, policy decisions, approvals, executions, verification,
and results. Secrets and unnecessary personal content are redacted. Audit data
is local, inspectable, bounded by retention policy, and resistant to silent
tampering within the user session.

## Initial repository shape

```text
shell/                 Quickshell/QML UI
core/
  broker/              capability routing
  policy/              permission evaluation
  executor/            sandboxed execution
  audit/               structured event records
  ipc/                 schemas and transport
ai/
  runtime/             provider-neutral orchestration
  providers/           llama.cpp, Ollama, future adapters
  planner/             plan and verification state machine
  tools/               structured tool definitions
system/
  context/             system and desktop context adapters
  privileged-helper/   minimal elevated operations
packaging/              Arch packages, services, ISO profiles
tests/                  cross-component and security tests
docs/decisions/         architecture decision records
legacy/                 migration home for the preserved prototype, if approved
```

Directories are introduced only when their first tested component exists. The
existing prototype remains in place until an ADR approves its relocation.

## First vertical slice

The first slice contains no LLM and performs one deterministic, harmless action:

1. The shell requests a registered diagnostic tool.
2. The broker resolves its required capability and scope.
3. Policy returns `ask`.
4. The shell displays the exact operation, reason, scope, and duration.
5. The user allows once or denies.
6. The executor runs the fixed diagnostic in a sandbox.
7. A verifier checks the exit status and expected result shape.
8. The shell displays the actual outcome.
9. Every transition appears in the audit history.

No implementation may bypass the broker for convenience.

## Change control

Material deviations from this contract require an accepted ADR and matching
updates to architecture, security, tests, and user documentation.
