# ADR-0016: Pinned offline llama.cpp evidence package

- Status: Accepted
- Date: 2026-09-03
- Accepted: 2026-09-03 after explicit approval of the Phase 4 production
  registry and deterministic package checkpoint
- Owners: Project maintainers

## Context

The provider-profile and readiness boundary cannot be completed using a moving
release, an installer that downloads at runtime, or a model name resolved by a
mutable registry. The first production package also needs a closed artifact set
small enough for repeatable CPU-only target-Linux evidence.

## Decision

The first release-compiled profile is `llama_cpp_cpu_v1` for x86-64 Linux. It
pins the official llama.cpp `b10775` Ubuntu x64 CPU archive and the
Qwen2.5-0.5B-Instruct Q4_K_M GGUF at repository revision
`9217f5db79a29953eb74d5343926648285ec7e67`. URLs, byte counts and SHA-256
digests are fixed. The package selects `llama-server`, required common
libraries and all packaged x86-64 CPU backends. Required library aliases become
measured regular files rather than runtime symlinks. Exact MIT and Apache-2.0
license inputs ship with the package tree.

Acquisition is outside the package builder. The builder accepts only
pre-fetched files, verifies every input before creating output, extracts only
named regular archive members and re-verifies each member. It never runs a
provider or model, enables a unit, creates an account, opens a socket or accesses
the network.

The canonical profile is generated from the reviewed lock and provider template
and embedded into `blossom-core` at compile time. A release build constructs
that opaque specification only on x86-64. Ollama and other architectures return
unavailable rather than falling back or accepting caller data.

## Consequences

- The package can be rebuilt offline once its exact inputs are present.
- Any runtime, model, architecture, file-set or license update needs review.
- This does not prove the Ubuntu binary runs on Arch or that live systemd
  isolation works. Those remain target-Linux evidence gates.
- The production gateway remains fail closed until it consumes installed
  readiness evidence and passes adversarial service tests.
