# ADR-0012: Provider endpoint identity and packaging boundary

- Status: Accepted
- Date: 2026-09-03
- Accepted: 2026-09-03 after explicit project review
- Owners: Project maintainers

## Context

ADR-0011 permits the Ollama and llama.cpp development adapters to send only
synthetic, developer-authored prompts to fixed loopback TCP ports. A loopback
address proves locality, not process identity. Another process running as the
desktop user can occupy the expected port and receive private prompts before
Blossom has any response to validate.

Checking a pathname, process name, PID, `/proc/<pid>/exe`, or port owner before
connecting does not bind the connection that carries the prompt. It also fails
under restart, PID reuse, replacement, and check-then-connect races. A bearer
token stored in the desktop user's environment or files would remain available
to the same-user attacker this boundary is meant to exclude.

Blossom therefore needs a production local-inference transport whose connected
peer is distinguishable from arbitrary desktop-user processes. The solution
must preserve provider replacement, keep provider responses untrusted, work
offline, and avoid turning model-runtime management into a privileged generic
execution service.

## Decision

### Trust split

Introduce a small `blossom-model-gateway` as the sole production ingress for
private inference. It is trusted only to:

- authenticate local client connections;
- validate the closed Blossom inference protocol and data classification;
- select one installed, code-owned provider profile;
- translate that request through an existing bounded provider adapter;
- normalize, validate, and return bounded stream events; and
- emit content-redacted operational evidence.

The gateway has no broker, approval, executor, filesystem-tool, shell,
privileged-operation, package-manager, model-download, or network-fallback
authority. It cannot execute a proposed tool intent. It runs as the dedicated,
static, non-login `blossom-model-gateway` system user.

The selected model provider runs separately as the dedicated, static, non-login
`blossom-model-provider` system user. It remains untrusted. It cannot bind the
gateway socket, read client credentials, reach user homes, reach the host or
external network, write audit state, or contact Blossom tools.

The desktop client and broker remain unprivileged user processes. Root, the
kernel, systemd, the installed gateway binary, and root-owned packaging are in
the trusted computing base for endpoint identity. A compromise of those
components remains outside this boundary, as already stated in
`SECURITY_MODEL.md`.

### Client-to-gateway transport

Production inference uses one fixed Linux `AF_UNIX` `SOCK_STREAM` pathname:

`/run/blossom-model-gateway/inference.sock`

The gateway service creates the socket inside its systemd-managed runtime
directory after dropping to `blossom-model-gateway`. The runtime directory and
socket are not writable by desktop users or by `blossom-model-provider`. The
socket grants connect access only to the explicit `blossom-ai` group. Abstract
Unix sockets are prohibited because they have no filesystem permission model.

After `connect` and before sending any request bytes, the client obtains
`SO_PEERCRED` from that same connected file descriptor and requires:

- the expected static `blossom-model-gateway` UID, resolved from root-owned
  system account data;
- a nonzero PID; and
- a GID consistent with the installed gateway account.

The client does not pre-check a PID or pathname and then trust a later
connection. Identity is read from the connection that will carry the request.
Failure to obtain or validate peer credentials closes the descriptor and sends
zero prompt bytes. PID is diagnostic correlation only; UID is the stable kernel
identity and PID reuse alone cannot satisfy the connected-peer check.

The gateway obtains `SO_PEERCRED` for every accepted client connection. It
requires a non-root real client UID, checks that UID's installed `blossom-ai`
group eligibility without trusting caller-supplied credentials, and binds every
request ID and cancellation message to that connection. Connections and
in-flight state are never shared across UIDs.

Socket permissions are defense in depth rather than the only authentication
mechanism. The client and gateway both validate kernel-supplied credentials.
Neither side accepts a caller-supplied UID, PID, GID, socket path, file
descriptor, credential structure, or authentication token. No reusable secret
is printed, logged, stored in an environment variable, or exposed to the model.

### Closed gateway protocol

The gateway protocol is a new closed, versioned, length-prefixed binary envelope
around the existing provider-neutral request and normalized stream event
schemas. It is not HTTP and is not a released public extension API.

Every frame has a code-owned maximum length, protocol version, request ID,
message kind, payload digest, and strict schema. Unknown fields, duplicate
fields, unknown message kinds, out-of-order events, invalid UTF-8, length
overflow, extra file descriptors, ancillary-data truncation, and frames after a
terminal event fail closed. One connection carries one inference request in the
initial implementation. Cancellation is scoped to that connection and request.

The gateway sends a bounded hello before accepting an inference request. It
identifies the gateway protocol version, selected installed provider profile,
current boot ID digest, and a fresh process-instance nonce. These values support
correlation and restart detection; they do not replace `SO_PEERCRED` and are not
secrets. The client rejects a profile or version it did not explicitly select.

The trusted client derives the input data classification. Synthetic requests
may continue to use the development adapters directly in test builds. Private
or ambient input is representable only in the gateway protocol and is rejected
unless the complete runtime identity and installed-profile validation described
by this ADR succeeded on the same connection.

### Provider isolation transport

The gateway and exactly one selected provider profile share a systemd-created
private network namespace anchored by a fixed Blossom namespace unit. Both
services set `PrivateNetwork=yes` and join that same namespace through
`JoinsNamespaceOf=`. The namespace has loopback only: no physical, virtual,
host, or external network interface, DNS, proxy, or default route.

Only inside that namespace, the gateway connects to the existing fixed provider
address and path:

- Ollama: `127.0.0.1:11434`, `POST /api/chat`;
- llama.cpp: `127.0.0.1:8080`, `POST /v1/chat/completions`.

The production client never connects to those ports. The provider cannot reach
the gateway's filesystem socket because it has a different UID and is not a
member of `blossom-ai`. The gateway calls only the inference endpoint and never
provider-native tool, agent, media, download, model-management, router, or
administrative endpoints.

The namespace anchor, gateway, and provider units have fixed ordering and
lifecycle dependencies. Loss or replacement of the namespace, gateway, or
provider connection terminates the request as a non-success. There is no direct
loopback, user-provider, remote, or cloud fallback.

### Installed provider profiles

The first production profiles are CPU-only evidence profiles, one each for
Ollama and llama.cpp. A profile is a root-owned, non-writable package manifest
that fixes:

- profile and provider kind;
- gateway protocol and adapter version;
- absolute provider binary and model paths;
- exact binary, model, manifest, and unit-file digests;
- exact executable arguments and environment allowlist;
- provider endpoint and inference path;
- filesystem and device access;
- process, memory, CPU, task, output, and deadline bounds; and
- expected static service UID/GID and systemd unit names.

The gateway accepts only profiles compiled into its closed registry and requires
the installed manifest to match that registry. It opens and hashes root-owned
regular files without following symlinks before marking a profile ready. Files
writable by either service identity or desktop users are rejected. Model
selection chooses among already installed reviewed profiles; it cannot supply a
path, URL, argument, environment variable, mount, device, unit, or digest.

Systemd starts the exact packaged provider command. Repository checks must keep
the manifest, unit, registry, and command line byte-consistent. Runtime evidence
must confirm the expected unit, static UID, private network namespace, read-only
artifact identity, and provider readiness before the gateway admits private
input. Any mismatch, unavailable evidence source, restart during validation, or
profile change invalidates readiness and sends zero private prompt bytes.

The initial checkpoint performs no download and installs no model. Real-model
CI obtains pinned artifacts in its controlled job, verifies their published
digests, disables external network access during execution, and records only
non-private evidence. Distribution download, update, rollback, and signature
policy remain Phase 9 work.

### Service hardening

Exact unit directives and resource values ship with the implementation and are
validated against the supported Arch systemd baseline. At minimum:

- both services use static non-login identities, `NoNewPrivileges=yes`, empty
  capability sets, syscall and address-family restrictions, private temporary
  directories, a read-only system view, and no access to home directories;
- the gateway can access only its runtime socket, installed manifests, required
  system identity evidence, and the shared private loopback namespace;
- the provider can read only its root-owned binary, libraries, selected model,
  and a narrowly declared disposable cache directory when required;
- provider writes are bounded and disposable; model and package artifacts are
  never writable by the provider;
- neither unit receives ambient secrets or a general shell environment;
- core dumps are disabled for both services; and
- task, memory, CPU, file-size, open-file, and execution-time limits are
  code-owned and fail closed.

GPU access is not part of the first identity checkpoint. Adding a device,
driver-specific library surface, GPU worker, or accelerator profile requires a
separate hardware threat review and profile-specific evidence.

### Logging, privacy, and failure

The gateway records request correlation, authenticated client UID digest,
gateway instance, selected profile digest, artifact-validation outcome,
provider kind, normalized terminal category, timing, and bounded token counts
when trustworthy. It omits prompt text, generated text, reasoning, tool
arguments, file paths, tool results, raw provider errors, group membership, and
model contents.

Operational logs cannot claim model correctness, provider trustworthiness,
provider-side cancellation, tool execution, or successful user intent. A
successful identity check means only that the exact connected gateway and its
installed provider profile satisfied this boundary at admission time.

### Explicit exclusions

This ADR does not add:

- private-input support by itself;
- an LLM planning loop or automatic tool execution;
- Bash, sudo, package management, model download, or graphical UI;
- caller-configurable endpoints or provider arguments;
- multi-user conversation state, shared prompt caches, or durable model memory;
- remote inference, LAN discovery, proxy use, redirects, or cloud fallback; or
- a security guarantee after root, kernel, systemd, or gateway compromise.

## Alternatives considered

### Continue fixed loopback TCP

Rejected for private input. Any desktop-user process can impersonate the
expected service at the familiar port.

### Check the port owner, PID, process name, or executable path

Rejected. Preflight checks do not bind the connection carrying the prompt and
are vulnerable to restart, PID reuse, replacement, and time-of-check/time-of-use
races.

### Authenticate with a bearer token in the user session

Rejected. A same-user process can commonly read user-owned files, inherited
environment, process state, or IPC and steal or replay the token. Token rotation
does not create process isolation.

### Use a per-user systemd service and Unix socket

Rejected for private input. Supervision improves lifecycle management but the
service and attacker retain the same Unix identity, so kernel peer credentials
do not distinguish them.

### Run the provider itself as the authenticated peer

Rejected. The large third-party provider would become part of the endpoint
authentication and client-protocol trusted computing base. The small gateway
keeps provider parsing and execution behind a separate identity and isolation
boundary.

### Link provider libraries into the desktop process

Deferred. It removes endpoint impersonation but places large native runtimes,
GPU backends, model parsers, and ABI lifecycle inside the desktop process. It
also weakens replaceability and crash isolation.

### TLS or application MAC over loopback

Rejected for the first local design. Key storage and rotation would recreate a
same-user secret problem. Kernel-authenticated Unix credentials plus distinct
service identities and a private provider namespace give a smaller local
boundary. Remote transport remains out of scope.

### D-Bus for token streaming

Rejected for the high-volume provider data path. D-Bus offers useful
credentialed control surfaces but would add message-bus policy and buffering to
the token stream. The narrow Unix protocol keeps one connected descriptor and
closed framing. No general IPC conclusion is made for future Blossom services.

## Security and privacy consequences

This design prevents an ordinary desktop-user process from receiving private
prompts merely by occupying a loopback port or filesystem path. The prompt is
sent only after the exact connected gateway has a distinct kernel-authenticated
UID and the code-owned provider profile passes runtime validation.

The gateway becomes security-sensitive code and must remain small. A gateway
memory-safety flaw, parser bug, packaging error, overbroad unit, or compromise
could expose private inference data. The provider still sees prompts and can
retain or infer information within its allowed memory and disposable storage.
Isolation reduces authority; it does not make the provider or model benign.

Group membership authorizes use of local inference, not access to Blossom tools
or another user's data. Root can impersonate credentials or replace packages;
that threat remains explicitly outside the current model.

## Operational consequences

Production private inference requires system packages, two static service
accounts, the `blossom-ai` access group, hardened system units, a namespace
anchor, root-owned manifests and artifacts, and target-Linux validation. The
current fixed-loopback adapters remain useful only for synthetic protocol tests.

CPU-only profiles are intentionally slow but reduce the first hardware and
device boundary. Provider upgrades, model changes, unit changes, and manifest
changes require new digests, conformance evidence, and protected review.

## Migration and rollback

Implementation proceeds in protected checkpoints:

1. closed gateway protocol, credential validation, and fake-provider fixtures;
2. package manifests, static identities, namespace anchor, and hardened units;
3. adapt Ollama and llama.cpp behind the gateway with synthetic input only;
4. adversarial target-Linux identity and isolation evidence;
5. narrowly enable private conversation input after the preceding gates pass;
6. cross-provider real-model offline evidence.

Until checkpoint 5, all private and ambient data remains rejected. Rollback
disables the gateway and provider units, removes private-input registration, and
returns both adapters to synthetic-only status. It never falls back to direct
loopback or remote inference.

## Validation

Before this ADR's roadmap item can be complete, tests must prove:

- zero request bytes are sent for wrong/missing peer credentials, socket
  replacement, gateway absence, malformed hello, wrong profile, wrong version,
  restart, PID reuse, or identity-service failure;
- desktop-user and provider-UID impostors cannot satisfy the gateway peer check
  or replace the socket;
- unauthorized client UIDs, forged credential fields, ancillary-data
  truncation, extra file descriptors, replay, cross-UID reuse, oversized frames,
  malformed schemas, and post-terminal frames fail closed;
- the provider namespace exposes loopback only and cannot reach the host, DNS,
  LAN, internet, user home, gateway socket, broker, executor, audit files, or
  privileged helper;
- binary, model, manifest, registry, unit, UID/GID, namespace, endpoint, and
  command drift blocks readiness before private bytes are written;
- provider absence, crash, restart, timeout, cancellation, output exhaustion,
  and gateway crash are bounded non-success outcomes;
- prompts, completions, reasoning, tool arguments, file content, raw errors, and
  model bytes are absent from logs and audit projections;
- the provider cannot call tools or configure endpoints, network, mounts,
  devices, processes, environment, model paths, or lifecycle operations;
- systemd security analysis and repository drift checks pass on the supported
  Arch baseline; and
- both pinned CPU evidence profiles complete equivalent normalized fixtures and
  real-model tests with external networking disabled.

The tests must exercise the production Linux credential, Unix-socket, systemd,
namespace, and filesystem paths. A synthetic mock alone is not exit evidence.

## References

- Linux Unix-domain socket and `SO_PEERCRED` semantics:
  <https://man7.org/linux/man-pages/man7/unix.7.html>
- systemd socket ownership and mode controls:
  <https://github.com/systemd/systemd/blob/main/man/systemd.socket.xml>
- systemd service execution and hardening controls:
  <https://github.com/systemd/systemd/blob/main/man/systemd.exec.xml>
- systemd `JoinsNamespaceOf=` semantics:
  <https://github.com/systemd/systemd/blob/main/man/systemd.unit.xml>
- systemd resource-control settings:
  <https://github.com/systemd/systemd/blob/main/man/systemd.resource-control.xml>
