# ADR-0017: Private gateway admission and cancellation

- Status: Accepted
- Date: 2026-09-03
- Accepted: 2026-09-03 after explicit project review
- Owners: Project maintainers

## Context

ADR-0012 selects a distinct-UID Unix gateway as the only production path for
private inference. The repository now validates one installed llama.cpp profile
at release startup, but intentionally creates no listener. Before that listener
can exist, Blossom needs one closed rule for who may connect, which request
fields a client may control, when private bytes may move, and how cancellation
races terminate.

Filesystem mode `0660` is useful defense in depth, but it does not prove that a
connected client remains eligible, bind a cancellation to its request, or stop
a pipelined second request. `SO_PEERCRED` exposes the peer's UID and primary GID;
membership in `blossom-ai` may instead be supplementary and must be resolved
from the same validated root-owned account snapshot used at admission.

## Decision

### Admission order

The production gateway performs these steps in order:

1. select the sole code-owned production profile for the build and architecture;
2. load the exact installed manifest and retain validated runtime, model, unit,
   `/etc/passwd`, and `/etc/group` descriptor evidence;
3. require its effective UID/GID to match the resolved gateway identity;
4. read and digest the boot ID once and create a fresh process nonce from the
   kernel random source;
5. create the fixed filesystem Unix socket with owner
   `blossom-model-gateway`, group `blossom-ai`, and mode `0660`, refusing any
   pre-existing pathname; and
6. accept a connection, obtain `SO_PEERCRED`, and authorize its non-root UID
   against the retained account snapshot before sending the hello or reading
   request bytes.

Failure at any step sends zero private bytes. No abstract socket, inherited
listener, socket activation, environment-selected path, profile fallback, or
caller-supplied identity is accepted.

### Client eligibility

An admitted UID must resolve exactly once in the retained passwd snapshot. It
is eligible when its passwd primary GID equals the resolved `blossom-ai` GID or
its exact account name occurs once in that group's bounded member list. Root,
the gateway identity, the provider identity, missing or duplicate accounts,
malformed group data, numeric caller claims, and changed account databases fail
closed. The kernel PID is retained only for redacted correlation and must be
nonzero.

Group membership grants access only to local inference. It grants no file,
broker, approval, tool, audit-journal, shell, sudo, D-Bus, or privileged-helper
authority.

### Private request schema

Add a distinct `PrivateInference` gateway frame. Its closed canonical payload
contains only the request ID, bounded conversation messages, minimized
code-owned intent catalogue, output mode, and deadline. It omits provider,
model, endpoint, path, classification, executable arguments, environment,
mounts, devices, and resource controls.

After admission, the gateway injects `private` classification plus the selected
profile's code-owned provider and logical model identity. The existing
`SyntheticInference` frame remains debug/test-only and is rejected by the
production listener. Direct provider adapters remain unable to construct
private requests outside this gateway decoding path.

One connection carries exactly one request. The gateway rejects pipelined
frames, a second request, cancellation before a request, a mismatched request
ID, and every frame after a terminal event. Request IDs are scoped to the
authenticated connection and cannot be replayed into another connection.

### Cancellation and lifecycle

Provider streaming and connection-frame reading proceed concurrently under one
request-scoped cancellation object. A valid matching `Cancel`, client EOF,
write failure, total deadline, provider disconnect, gateway shutdown, or loss
of the namespace/provider causes a non-success terminal state. Cancellation
wins over a completion that has not already been validated and committed as the
single terminal event.

The gateway does not claim that the provider erased already received bytes or
stopped computation. It stops releasing output, closes the provider
connection, emits at most one bounded terminal event when the client remains
connected, and discards all partial tool arguments. No proposed intent is
released after cancellation.

### Output and audit boundary

Every provider event passes the existing normalized stream validator before it
is written to the client. Tool intents remain proposals and receive no path to
the broker in this service.

Operational evidence may contain request-ID and UID digests, gateway instance,
boot/profile/artifact digests, bounded timing and trustworthy token counts, and
terminal category. It must omit UID/account names, PID, group membership,
prompt/generated/reasoning text, tool arguments/results, paths, model contents,
raw frames, and raw provider errors. Evidence write failure cannot turn a
failed or indeterminate request into success.

## Alternatives considered

### Authorize only with socket permissions

Rejected. Permissions neither authenticate the connected descriptor in code
nor support reviewable supplementary-membership and request-binding evidence.

### Accept provider, model, or classification from the client

Rejected. These fields could widen the installed authority and make `private`
an untrusted wire assertion. The gateway derives them from its admitted profile.

### Process a request synchronously and check cancellation afterward

Rejected. A blocked provider read would make cancellation and client disconnect
ineffective and could release a completion that lost the race.

### Permanent sessions or multiple requests per connection

Deferred. They require more complex identity, replay, cancellation, memory, and
resource accounting. Phase 4 uses one connection per request.

## Security and privacy consequences

The ordinary desktop-user threat is constrained by both explicit group
eligibility and kernel credentials, while provider selection and isolation stay
outside caller control. Any process running as an already authorized UID may
use local inference and submit content available to that UID; this boundary
does not isolate mutually untrusted processes sharing one Unix account.

Root, kernel, systemd, gateway, and root-owned packages/account databases remain
in the trusted computing base. The provider remains untrusted and receives the
private prompt after admission. Isolation limits its authority but cannot make
the model confidential from the provider process itself.

## Operational consequences

The package must install exactly one rendered gateway/provider profile, create
the named accounts, and opt users into `blossom-ai`. Account changes require a
gateway restart because admission uses its retained snapshot. The service is
not socket-activated and refuses stale socket paths; systemd runtime-directory
cleanup owns recovery after a clean stop.

## Migration and rollback

Implement the private frame and membership parser under tests first. Then add a
production listener that remains package- and CI-gated. Rollback disables the
gateway service and removes the runtime-directory socket; synthetic adapters
remain test-only and no direct private-provider fallback is introduced.

## Validation

Before private input is enabled, Linux evidence must prove:

- exact socket path, type, owner, group, mode, stale-path refusal and cleanup;
- zero request bytes for unauthorized, root, malformed, changed or duplicate
  account identities;
- primary and supplementary access-group admission from retained data;
- hello and peer identity binding on the connected descriptor;
- rejection of synthetic, noncanonical, oversized, pipelined, second-request,
  wrong-ID cancellation and post-terminal frames;
- cancellation during connect, headers, streaming and completion races, with no
  completed intent after cancellation;
- bounded deadlines, output, concurrency and connection count;
- provider/namespace loss and audit failure as non-success; and
- installed distinct-UID services, loopback-only namespace, filesystem denials,
  external-network denial, and real offline inference for both providers.

Phase 4 remains active until that production-path evidence and the separate
pinned Ollama package evidence are merged.
