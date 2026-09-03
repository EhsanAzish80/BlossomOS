# Local model runtime package boundary

Status: reviewed templates only. Nothing in this directory is installed,
enabled, started, or rendered by the repository test suite.

## Static identities

`blossom-model-runtime.sysusers` declares two persistent, non-login system
accounts and the opt-in `blossom-ai` access group:

- `blossom-model-gateway` owns the authenticated Unix-socket gateway;
- `blossom-model-provider` runs exactly one selected untrusted provider; and
- membership in `blossom-ai` authorizes connection to local inference only.

The sysusers configuration deliberately lets the target system allocate
collision-free numeric IDs. “Static” means named, persistent accounts rather
than `DynamicUser=` identities. The future gateway must resolve the installed
root-owned account records, bind the resolved IDs to its profile/readiness
evidence, and reject missing, root, shared, or changed identities.

## Fixed paths

| Purpose | Fixed path | Intended owner and mode |
| --- | --- | --- |
| gateway binary | `/usr/lib/blossom-os/blossom-model-gateway` | `root:root 0755` |
| manifests | `/etc/blossom-os/model-profiles/` | `root:root 0755`; files `0644` |
| gateway socket | `/run/blossom-model-gateway/inference.sock` | `blossom-model-gateway:blossom-ai 0660` |
| provider runtime set | rendered package directory under `/usr/lib/blossom-os/providers/` | root-owned, non-writable; executable `0755` |
| models | rendered absolute package path under `/usr/lib/blossom-os/models/` | `root:root 0644` |
| disposable provider state | `/var/lib/blossom/model-provider/<profile>/` | `blossom-model-provider:blossom-model-provider 0700` |

The service creates the runtime directory, but the future gateway binary—not a
socket unit—must create the socket with the exact group and mode and then
authenticate every connected peer. No abstract or caller-selected socket is
part of this boundary.

## Units

- `blossom-model-netns.service` anchors a private network namespace.
- `blossom-model-gateway.service` joins it and is the only private-input ingress.
- The two `.service.in` files are provider-specific render templates. A future
  package step must replace every allowlisted token with reviewed profile data,
  verify the rendered unit, hash it, and bind that digest into the closed
  registry and installed manifest.

There is intentionally no generic instance unit and no caller-controlled `%i`.
The units have no `[Install]` section, so this checkpoint cannot enable them.
The templates are CPU-only and expose no device. Both provider templates allow
only loopback inside the shared private namespace; neither has a route, DNS,
proxy, host network, Unix-socket, home-directory, shell, tool, or broker path.
The Blossom package tree is otherwise inaccessible and only the complete
measured provider runtime directory and model are rebound read-only; ordinary dynamic-loader and system-library
paths remain visible because the provider cannot start without them.

## Not implemented

The gateway process source now exists, but its production entry point exits
not-ready before creating a listener. A debug/test-only renderer produces fixed
synthetic units for CI; it is not a production renderer or installed tool. The
core now has a fail-closed validator for installed account and artifact evidence,
but no package invokes it because the production registry remains absent. A
package recipe, installed release binary/manifests, provider/model artifacts,
account creation, socket creation, service activation, private input, and real
model execution remain absent. Template and synthetic readiness validation are
not production isolation evidence and do not complete ADR-0012 or Phase 4.
