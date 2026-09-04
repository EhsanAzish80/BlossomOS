# ADR-0019: Pinned offline Ollama package and active profile selection

- Status: Accepted
- Date: 2026-09-04
- Accepted: 2026-09-04 as required to complete the approved Phase 4 boundary
- Owners: Project maintainers

## Context

ADR-0011 requires replaceable local providers, ADR-0012 requires exactly one
installed provider behind the authenticated gateway, and ADR-0013/0015 require
closed model and runtime artifact sets. The llama.cpp package satisfies that
boundary for one provider. Ollama remains synthetic-only, while production
gateway startup is hard-coded to the llama.cpp manifest path and adapter.

Ollama's ordinary pull and create flows are unsuitable for this package. They
introduce network access, mutable registry resolution, runtime-generated store
state, and a wider executable surface. The official Linux archive also bundles
GPU runtimes that a CPU profile neither needs nor may expose.

## Decision

Blossom adds one x86-64 Ollama CPU package profile with these immutable inputs:

- official Ollama `v0.33.3` Linux amd64 release archive and tagged MIT license;
- the exact registry response bytes for
  `qwen2.5:0.5b-instruct-q4_K_M`; and
- every config, model, system, template, and license blob named by that
  manifest, fetched only through its content-addressed digest.

The source lock records every URL, byte size and SHA-256 digest. Package
assembly accepts only pre-fetched regular files, verifies the archive and every
selected member, rejects unknown or missing model blobs, and constructs the
canonical Ollama store without running Ollama. It copies only the executable,
CPU runner libraries and license material required by the profile. CUDA,
ROCm, Vulkan, MLX and other device runtimes are excluded. The installed service
sets the code-owned CPU library selection and has no device access or external
network route.

Both production profile specifications are compiled from canonical repository
bytes. A package installs exactly one profile at the fixed, root-owned regular
file `/etc/blossom-os/model-profiles/active.json`. Startup reads that file once
through the existing retained readiness boundary and accepts it only if its
exact bytes match one embedded specification. The caller cannot select a
provider, model, endpoint, path or adapter. The gateway dispatches the adapter
only from the validated active profile.

The llama.cpp package migrates to the same active path. No package may install
both provider units or more than one active profile. Services remain disabled
after package assembly and installation.

## Alternatives

### Run `ollama pull` during installation or first boot

Rejected. It makes installation depend on mutable remote state and gives the
provider network authority before the local boundary is established.

### Run `ollama create` while assembling the package

Rejected. The package builder can create the documented content-addressed store
directly and verify every byte without executing the provider.

### Package the complete upstream GPU archive

Rejected. It adds more than a gigabyte of unused device runtimes and expands
the measured and executable attack surface of a CPU-only profile.

### Let a command-line flag or environment variable select a provider

Rejected. Those are caller-controlled authority. Selection belongs to the one
root-owned canonical active profile installed by the package.

## Security and privacy consequences

The provider still receives admitted private prompts and remains untrusted.
Exact package identity, distinct service UIDs, the private loopback namespace,
filesystem denial, resource bounds, output validation, cancellation, and
content-free audit remain mandatory. A digest proves which bytes were packaged;
it does not establish upstream trust or model safety.

The registry tag URL identifies the reviewed model name but is not trusted for
immutability: the downloaded manifest must match the locked byte size and
digest, and its complete referenced blob set must match the lock. Runtime has no
registry or general network path and cannot fetch missing content.

## Validation

Before the Ollama profile is called supported, reviewable Linux evidence must
prove:

- deterministic package assembly and canonical registry drift checks;
- exact CPU-only runtime and complete manifest/blob inventories;
- active-profile matching with unknown, duplicate, symlinked and modified
  profiles failing before socket creation;
- installed distinct identities, mounts, namespace, resource limits and
  admission denials equivalent to llama.cpp;
- real offline inference, schema normalization, provider loss, cancellation,
  terminal-write containment and audit behavior; and
- successful evidence on the declared target architecture.

Phase 4 remains active until this package and the separate target-Arch evidence
are merged. Phase 5 must not treat provider output as authorization.

## Migration and rollback

The llama.cpp recipe installs its unchanged canonical profile as `active.json`.
Rollback stops the gateway and provider, installs one previously reviewed
active package, and restarts the namespace, provider and gateway in order. It
never falls back to another provider automatically.
