# Blossom OS Security Model

Status: architectural security contract. Phase evidence documents identify the
small subset implemented and tested; this contract must not be read as claiming
that the preserved prototype or target desktop already satisfies every boundary.

## Assets to protect

- User files, credentials, identity, clipboard, notifications, and activity.
- System integrity, boot configuration, packages, services, devices, and network.
- Permission decisions, approval intent, audit history, and model configuration.
- The boundary between untrusted model output and executable operations.

## Threat actors and failure modes

- Malicious or compromised model output.
- Prompt injection in files, web content, tool output, or notifications.
- A compromised shell, plugin, model provider, or unprivileged Blossom service.
- Confused-deputy requests and forged or replayed approvals.
- Command injection, path traversal, symlink races, environment poisoning, and
  sandbox escape.
- Supply-chain compromise or a malicious update.
- Accidental overbroad actions caused by ambiguous user intent.

The initial threat model does not promise protection after the kernel, root
account, firmware, or physical machine is already compromised.

## Trust boundaries

Model text, plans, external content, repository files, and tool output are
untrusted data. They are never authorization.

ADR-0011 also treats a local model provider as untrusted. A loopback address is
not provider authentication. Until the connected provider instance is bound to
a reviewed code-owned service identity, real adapters may receive synthetic
test prompts only and must reject private or ambient user data. Model-visible
tool catalogues are empty by default and minimized per turn; model calls remain
untrusted proposals outside the internal request and approval types.

ADR-0012 proposes a distinct-UID Unix-socket gateway and a separately isolated
provider service as that identity boundary. The proposal is not accepted or
implemented. Direct loopback adapters therefore remain synthetic-only, and no
current model path is authorized to receive private or ambient user data.

The shell may collect approval but may not forge policy decisions. The broker and
policy engine run without root. The executor receives only an approved,
normalized request. The privileged helper independently validates a narrow typed
operation and its fresh authorization evidence.

## Capability model

Capabilities use a verb and scope, for example:

```text
system.read
files.read:/home/user/Documents/project
files.write:/home/user/Downloads
apps.launch:org.example.App
shell.execute:/usr/bin/git
packages.install:repo-package-name
services.restart:bluetooth.service
system.modify:time-zone
privilege.elevate:operation-id
```

Capabilities are deny-by-default. Wildcards require an ADR and a threat review.
Tools declare their capabilities statically; runtime arguments may only narrow
scope.

ADR-0004 makes typed capability values authoritative for Phase 2, requires exact
resource scopes, fixes the initial expansion order, and prohibits callers from
configuring execution or sandbox profiles. Every added tool receives an
independent privacy and cross-tool composition review.

Policy decisions are `allow`, `deny`, or `ask`. Approval grants may be once-only,
time-bounded session grants, or explicit persistent rules. A model cannot create,
expand, or persist a grant.

## Approval requirements

An approval prompt must show:

- What will happen in human-readable and exact structured form.
- Why it is requested and which user request caused it.
- Required privilege, filesystem/network scope, and expected side effects.
- Whether the grant is once-only, session-scoped, or persistent.

Changing the request invalidates approval. High-risk or privileged operations do
not support ambiguous bulk approval.

## Execution controls

The executor must support:

- No shell parsing by default; use argument arrays and absolute executables.
- Allowlisted environment variables and a known working directory.
- Filesystem scopes, network policy, timeouts, resource/output limits, and process
  isolation.
- Captured exit status, stdout, stderr, signal, and termination reason.
- Verification separated from execution.

ADR-0003 selects Bubblewrap for the fixed Phase 1 diagnostic and records its
tested guarantees and omissions. systemd-run, seccomp, Landlock, and polkit
remain candidates for later scoped decisions and target-Arch validation.

Raw shell is a fallback capability, never the default tool interface. A generic
privileged shell is prohibited.

## Privileged helper rules

- Expose closed, versioned operations rather than commands.
- Run with the minimum Linux capabilities and filesystem access.
- Revalidate caller, operation, arguments, policy decision, approval freshness,
  and replay protection.
- Reject unknown fields, out-of-scope paths, symlinks where unsafe, and stale or
  reused authorization.
- Produce an audit event before and after the operation.

## Audit and privacy

Audit records include correlation ID, actor, tool, normalized arguments or a
redacted digest, capabilities, policy result, approval, execution metadata,
verification, and outcome. Tokens, model secrets, file contents, and credentials
must not be logged by default.

Users can inspect, export, and delete their local history subject to clearly
explained integrity and retention behavior. No telemetry or remote inference is
enabled silently.

## Legacy findings

The preserved prototype includes scripts advertising a default username and
password of `blossom` and does not implement the documented sandbox or permission
system. Those scripts are not a secure distribution baseline and must not ship
unchanged. Their presence is preserved for history, not endorsed.

## Required security tests

- Deny-by-default and scope narrowing.
- Approval binding, expiry, replay rejection, cancellation, and mutation.
- Command/argument injection and environment filtering.
- Path traversal, symlink, race, and filesystem-scope tests.
- Timeout, output, process, resource, and network containment.
- IPC authentication, schema rejection, size limits, and version behavior.
- Privileged-helper negative tests for every operation.
- Audit redaction, completeness, correlation, and retention.
- Fail-closed behavior when any security service is unavailable.

Security boundaries must never rely only on system prompts or model compliance.
