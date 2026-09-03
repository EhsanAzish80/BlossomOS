# Phase 4 pinned llama.cpp registry and package checkpoint

Status: one x86-64 llama.cpp profile is release-constructible and has a
deterministic offline package-tree recipe. This is not installed-service or
real-inference evidence, and the production gateway remains disabled.

## Implemented

- `system/model-runtime/registry/llama-cpp-cpu-x86_64.lock.json` pins immutable
  upstream runtime, model and license inputs by URL/revision, size and SHA-256.
- Its canonical profile binds schema v4, the complete normalized runtime set,
  Qwen GGUF, rendered unit, identities, endpoint, arguments, filesystem scope
  and resource limits.
- `production_provider_profile` parses only compile-time embedded canonical
  bytes. It exposes this profile only on x86-64 and returns unavailable for
  Ollama or unsupported architectures.
- `scripts/package_llama_cpp_runtime.py` performs no download. It verifies all
  inputs before output creation, selects only pinned regular archive members,
  replaces archive aliases with measured regular copies, includes exact
  upstream licenses, renders the fixed unit and creates no enablement links.
- The package receipt binds the canonical profile, supplied gateway build and
  source lock, and records that services are not enabled.

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

This evidence was produced on the development host. The x86-64 Linux provider
was not executed there. It does not satisfy target-Arch ABI, systemd, namespace,
peer-authentication, network-isolation or real-inference exit requirements.

## Remaining

- build and validate the deterministic Ollama runtime/model-store package;
- consume installed llama.cpp readiness evidence in the production gateway;
- install the package into the supported target-Linux test environment;
- prove identities, lifecycle, socket authorization, filesystem denials and
  loopback-only networking adversarially; and
- record real offline inference through the authenticated gateway for both
  providers before Phase 4 can exit.
