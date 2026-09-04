# Local model runtime package boundary

Status: reviewed templates and two deterministic evidence-package recipes.
Nothing is installed or enabled merely by cloning or testing the repository;
manually dispatched disposable Linux workflows assemble, install, start and
adversarially test the pinned evidence packages.

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
- `blossom-model-gateway.service.in` renders the selected profile's exact
  manifest, runtime directory, and model path into read-only binds, joins the
  namespace, and is the only private-input ingress.
- The provider `.service.in` files and gateway template are profile-specific.
  The llama.cpp package recipe replaces every allowlisted token with its closed
  registry data; future provider recipes must apply the same rule. The provider
  unit is hashed and bound into the registry and installed manifest.

There is intentionally no generic instance unit and no caller-controlled `%i`.
The units have no `[Install]` section, so this checkpoint cannot enable them.
The templates are CPU-only and expose no device. Both provider templates allow
only loopback inside the shared private namespace; neither has a route, DNS,
proxy, host network, Unix-socket, home-directory, shell, tool, or broker path.
The Blossom package tree is otherwise inaccessible and only the complete
measured provider runtime directory and model are rebound read-only; ordinary dynamic-loader and system-library
paths remain visible because the provider cannot start without them.

## Current boundary

The gateway production listener exists only behind the non-default
`production-private-inference` feature and fails closed unless the exact active
profile, installed accounts, package artifacts, rendered unit and audit state
validate. ADR-0016 and ADR-0019 define pinned CPU-only llama.cpp and Ollama
x86-64 evidence packages. Disposable Ubuntu workflows have installed and tested
each package with real offline inference, and a pinned Arch userspace workflow
has built/tested the Rust boundary and assembled/inspected both package roots.

These recipes are not a package repository, installer, updater, model manager,
ArchISO, supported release, or general provider loader. Default builds still
open no private inference listener.
