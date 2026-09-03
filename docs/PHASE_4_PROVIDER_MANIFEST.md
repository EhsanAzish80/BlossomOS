# Phase 4 provider-profile manifest checkpoint

Status: implemented as a closed validator and synthetic filesystem fixture. It
is not a packaged provider, an installed manifest, or the Phase 4 exit baseline.

## Implemented boundary

The core now defines a versioned, deny-unknown-fields provider profile covering
the provider/profile pair, protocol versions, exact binary and model artifacts
and SHA-256 digests, service-unit digest, executable arguments, environment-name
allowlist, fixed endpoint and inference path, filesystem visibility, resource
bounds, and static service identities.

The installed-manifest loader requires an absolute path and, on Linux, opens it
relative to `/` with `openat2`, `RESOLVE_BENEATH`, `RESOLVE_NO_MAGICLINKS`, and
`RESOLVE_NO_SYMLINKS`. It accepts only a root-owned regular file with no group or
world write bit, bounds the descriptor read, compares descriptor metadata before
and after reading, and records the device, inode, byte count, and digest of the
exact bytes parsed. Other Unix development hosts reject a final symlink with
`O_NOFOLLOW`; target-Linux CI supplies the stronger complete-path evidence.

Authority does not come from the file. Loading requires an opaque
`ProviderProfileSpec` containing independently code-owned canonical bytes and a
derived digest. This checkpoint intentionally provides no production constructor
for that specification: packaging must add a closed compiled registry after the
actual packaged artifact and unit digests exist. A caller or future model cannot
turn arbitrary manifest data into an accepted specification.

## Closed CPU-only policy

- Ollama and llama.cpp profiles have fixed loopback endpoints, inference paths,
  provider-specific unit names, gateway unit, and namespace-anchor unit.
- Binary and model paths are the complete read-only filesystem allowlist.
- Writable paths must be absolute and disjoint from those artifacts.
- Device access is empty, so this checkpoint does not authorize a GPU.
- Endpoint override arguments, URLs, malformed paths, secret-bearing environment
  values, invalid identities, and out-of-range resources are rejected.
- The manifest cannot contain its own digest; the expected specification derives
  it from canonical bytes.

## Evidence

Unit tests cover canonical loading and provenance, byte modification and
non-canonical encoding, unknown fields, remote endpoints, endpoint override
arguments, devices, writable-model expansion, wrong unit identity, unsafe mode,
wrong owner, final symlinks, directories, and oversized input. Linux-only tests
also reject a symlinked parent component.

## Deliberately absent

- root-owned installed files or a production profile registry;
- model or provider binaries, downloads, package metadata, or system accounts;
- systemd units, namespace creation, provider lifecycle, or real provider I/O;
- GPU access, private or ambient input, shell access, or broker authority; and
- a claim that ADR-0012 or Phase 4 is complete.

## Next checkpoint

The package-owned identity, filesystem, namespace-anchor, and hardened systemd
template boundary is now defined in `PHASE_4_SYSTEMD_BOUNDARY.md`. Next,
implement the small gateway process for synthetic input only. A later package
checkpoint must bind a closed registry to real artifact and rendered-unit
digests before any production listener or private input is enabled.
