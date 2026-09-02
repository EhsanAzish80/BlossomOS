# ADR-0003: Bubblewrap for the Phase 1 Linux sandbox adapter

- Status: Accepted
- Date: 2026-09-02
- Owners: Project maintainers

## Context

Phase 1 needs one real, unprivileged Linux execution path for a fixed diagnostic.
The portable broker and policy core must remain testable on macOS, and sandbox
failure must never fall back to direct host execution.

Bubblewrap constructs an empty mount namespace and lets the caller expose only
explicit filesystem paths. It can also isolate user, PID, IPC, network, UTS, and
cgroup namespaces. systemd-run offers useful service and resource controls but is
not a replacement for the explicit filesystem view required by this checkpoint.

## Decision

Use distribution-packaged Bubblewrap for the Phase 1 Linux executor adapter.
The adapter:

- supports only the fixed `/usr/bin/uname -s` diagnostic;
- invokes argv directly and never invokes a shell;
- unshares all supported namespaces, including network;
- exposes `/usr` read-only and creates only minimal `/proc`, `/dev`, and temporary
  `/tmp` views;
- clears the environment and re-adds only validated entries;
- drops all capabilities, disables nested user namespaces, starts a new session,
  and requests child death with the parent;
- applies timeout and combined output limits outside the sandbox;
- fails closed if Bubblewrap is absent, rejects the specification, or cannot
  establish the namespace.

No unsandboxed fallback is permitted.

## Alternatives considered

- systemd-run: valuable later for cgroup resource policy and service lifecycle,
  but insufficient alone for the minimal explicit filesystem view.
- Direct namespaces/seccomp: finer control, but substantially more privileged and
  error-prone code than this checkpoint warrants.
- Containers: excessive orchestration and daemon attack surface for one local
  diagnostic.

## Security and privacy consequences

The adapter hides the user's home and host filesystem except for read-only
`/usr`, and removes network access. Bubblewrap remains part of the trusted
computing base. Namespace availability and distribution policy can prevent it
from starting; that condition is an execution failure, not permission to run on
the host.

This checkpoint does not add seccomp, Landlock, cgroup resource controls, or
writable filesystem scopes. Those remain Phase 2 decisions.

## Operational consequences

Linux test and target systems require the `bubblewrap` package. macOS runs the
portable core tests and returns `Unavailable` from the Linux adapter. CI installs
Bubblewrap and runs the real integration test on Ubuntu.

GitHub's Ubuntu runner restricts unprivileged user namespaces through AppArmor by
default. The repository check enables them only inside the disposable CI runner
before exercising Bubblewrap, matching Bubblewrap upstream's own CI setup.
Blossom does not change this host setting at runtime.

## Migration and rollback

The adapter is behind the existing executor trait. It can be replaced by
superseding this ADR without changing request, policy, approval, verification, or
audit types.

## Validation

- Unit tests reject command, argument, environment, timeout, output, and network
  expansion.
- Linux CI executes the fixed diagnostic inside Bubblewrap.
- CI inspects the generated argv for namespace, read-only bind, environment, and
  no-network flags.
