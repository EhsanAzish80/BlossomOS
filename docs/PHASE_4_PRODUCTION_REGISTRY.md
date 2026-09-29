# Phase 4 pinned llama.cpp registry and package checkpoint

Status: x86-64 and aarch64 llama.cpp profiles are release-constructible from
one shared model record and architecture-specific runtime records. The
aarch64 package has passed an installed-root receipt check through `pacman -U`;
the earlier x86-64 profile retains installed-service real-inference evidence.
The production gateway remains disabled by default.

## Implemented

- `system/model-runtime/registry/qwen2.5-0.5b-instruct-q4_k_m.model.json` pins
  the shared model and license. Separate `llama-cpp-b10775-x86_64.runtime.json`
  and `llama-cpp-b10775-aarch64.runtime.json` records pin the executable runtime;
  the aarch64 record also binds the upstream attestation ID and URL.
- Each canonical profile binds schema v5, its architecture, the complete runtime set,
  Qwen GGUF, rendered unit, identities, endpoint, arguments, filesystem scope
  and resource limits.
- `production_provider_profile` parses only compile-time embedded canonical
  bytes selected for the compiling architecture. Ollama remains x86-64-only.
- `scripts/package_llama_cpp_runtime.py` requires an explicit architecture and
  performs no download. It verifies all
  inputs before output creation, selects only pinned regular archive members,
  replaces archive aliases with measured regular copies, includes exact
  upstream licenses, renders the fixed unit and creates no enablement links.
- The package receipt binds the canonical profile, supplied gateway build and
  the separate model and runtime records, and records that services are not
  enabled. `blossom-core` owns the gateway binary; the runtime package depends
  on it and verifies its digest without claiming the same filesystem path.

## Aarch64 package evidence recorded on 2026-09-28

The official 13,362,143-byte aarch64 archive matched SHA-256
`263d4995214ae1cf9cfb19f07b9d995b8192ab82d4dbe9734332e9348459290d`
and upstream attestation ID `44875193`. The shared Qwen model retained the
same bytes and digest as the x86-64 profile.

The first package was truthfully rejected by pacman because it duplicated the
gateway path already owned by `blossom-core`. After correcting ownership, fresh
aarch64 `blossom-core` and `blossom-model-runtime` packages installed together
with `pacman -U`. Installed-root receipt verification passed, pacman reported
0 altered files for both packages, and the gateway remained owned only by
`blossom-core`. This is package and receipt evidence, not inference evidence.

## Evidence recorded on 2026-09-03

The official 16,718,980-byte llama.cpp archive matched SHA-256
`faac52e16e5749713d33531ab7e4161fd0f09e7f2dccb4ed7527162d4c3bd103`.
The 491,400,032-byte Qwen GGUF matched
`74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db`.
Both pinned license inputs also matched their lock entries.

Two package trees built from the same inputs compared byte-for-byte equal and
contained no symlinks. A wrong archive was rejected before output creation. The
embedded registry passed canonical Rust validation and the release workspace
compiled.

The package inputs and deterministic-tree evidence above were produced on the
development host. A later disposable x86-64 Linux run installed that package
and passed systemd identity, namespace, peer-admission, network-isolation,
audit and real offline-inference checks; see
`docs/PHASE_4_INSTALLED_EVIDENCE.md`. This Ubuntu-runner evidence is not yet a
target-Arch ABI claim.

## Remaining

- build and validate the deterministic Ollama runtime/model-store package;
- add target-Arch package/ABI evidence;
- complete the remaining installed ADR-0017/0018 adversarial cases, including
  broader filesystem denial; and
- record real offline inference through the authenticated gateway for Ollama
  before Phase 4 can exit with both providers supported.
