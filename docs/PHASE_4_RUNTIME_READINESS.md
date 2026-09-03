# Phase 4 runtime-readiness checkpoint

Status: implemented as a fail-closed validation primitive. This is not an
installed package, active service, production registry, or Phase 4 exit
baseline.

## Bound evidence

`load_installed_runtime_readiness()` first loads the canonical root-owned
provider manifest, then resolves the fixed `blossom-model-gateway`,
`blossom-model-provider`, and `blossom-ai` identities from `/etc/passwd` and
`/etc/group` without NSS, a subprocess, or a shell. It requires:

- unique, non-root, distinct gateway and provider users and primary groups;
- `/` homes and `/usr/bin/nologin` shells;
- gateway membership in the distinct `blossom-ai` access group; and
- no provider membership in that access group.

The canonical manifest fixes the account names. Their resolved gateway/provider
numeric IDs, account-database digests, sizes, devices and inodes are retained as
readiness evidence; the access-group ID is also recorded.

The validator inventories and opens every provider-runtime file, including the
exact executable, every model-set file and
rendered provider unit beneath `/` with no symlink or magic-link traversal on
Linux. For Ollama it first enumerates the root-owned, non-writable store and
requires exact equality with the canonical artifact list; unknown files,
symlinks and special entries fail closed. Every measured file
must be regular, root-owned, not group/world writable, within a code-owned size
limit and unchanged throughout its descriptor read. The executable must have an
execute bit. Streaming SHA-256 must match the canonical manifest's artifact and
unit digests.

The validated runtime-set, model-set and unit descriptors remain owned by the readiness
object. A future launcher must consume these pinned descriptors; reopening the
paths would reintroduce a path-replacement TOCTOU race and is not authorized by
this checkpoint.

## Evidence

Portable tests cover valid account resolution and reject root, shared, login,
duplicate and over-broad access-group identities. An end-to-end synthetic
filesystem tests bind account records, the canonical manifest, executable,
model set and rendered unit, check their digests and retain descriptor identity.
Linux CI supplies the `openat2` path-containment evidence; the ordinary test and
lint matrix continues to validate other supported build hosts.

## Deliberately absent

- a release-constructible production profile registry;
- package installation, account creation, service activation or provider launch;
- socket creation, listener readiness, health checks or namespace membership;
- private prompts, ambient data, real models, downloads or GPU access.

The release gateway still exits not-ready before opening a socket. Phase 4
remains active until the remaining ADR-0012 production-path and offline
real-model target-Linux evidence is produced.
