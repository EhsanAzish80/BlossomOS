# ADR-0013: Closed model artifact sets

- Status: Accepted
- Date: 2026-09-03
- Accepted: 2026-09-03 after explicit project-owner review
- Owners: Project maintainers

## Context

ADR-0012 requires the production profile to bind the exact installed model
artifact before private input is admitted. The initial manifest represented a
model as one file. That matches llama.cpp's GGUF input but not Ollama's model
store, which contains a manifest and content-addressed blobs beneath a model
root. Hashing one placeholder path would leave other provider-consumed bytes
outside the readiness proof.

## Decision

Provider profile version 2 replaces the single model artifact with:

- one exact `model_mount` exposed read-only to the provider;
- a non-empty, strictly sorted, duplicate-free list of absolute model files;
- an exact SHA-256 and code-owned size bound for every regular file; and
- a canonical SHA-256 binding the complete ordered artifact list.

For llama.cpp, the mount and the only listed file are the same GGUF. For
Ollama, the mount is the immutable model-store root and every manifest and blob
the selected profile can consume is listed. All listed files must be strict
descendants of that root.

At readiness, trusted code enumerates the Ollama root and requires exact set
equality. Unknown files, missing files, symlinks, special files, traversal,
unsafe directory ownership or permissions, individual digest drift, aggregate
set drift, count overflow and total-size overflow fail closed. Each validated
file is opened once, hashed through that descriptor and retained for the
readiness lifetime. Root compromise remains outside the threat model.

Model selection remains a closed profile choice. Callers and models cannot add
paths, files, digests, mounts or provider configuration. Package creation must
generate the canonical list from reviewed pinned inputs and repository checks
must bind it to the unit and registry.

## Alternatives considered

### Hash only the primary Ollama manifest

Rejected because referenced blobs could be replaced without invalidating
readiness.

### Hash a directory pathname or metadata

Rejected because directory metadata does not authenticate file contents or
exclude unknown entries.

### Package the store as a tar archive

Rejected because extraction creates another mutable, unbound filesystem state
and would require additional trusted lifecycle authority.

### Use one mounted filesystem image

Deferred. It could reduce the artifact surface but introduces image creation,
mount lifecycle and privileged setup that are unnecessary for the first CPU
evidence profiles.

## Security and privacy consequences

The provider cannot consume unmeasured model-store content without readiness
failing. The larger manifest and descriptor set increases validation cost and
file-descriptor pressure, so count and total-byte bounds are mandatory. This
decision authenticates installed bytes; it does not make a model safe, prevent
in-memory retention, or protect against root, kernel or gateway compromise.

## Migration and rollback

Version-1 manifests are rejected after this change. Synthetic fixtures and the
future production registry migrate together to version 2. Rollback requires a
reviewed code rollback and never accepts both schemas simultaneously.

## Validation

Tests must cover deterministic set digests, ordering, duplicates, missing and
unknown files, symlinks, special files, unsafe ownership and modes, path escape,
individual and aggregate digest drift, file-count and total-size bounds, and
descriptor retention. Real-model evidence must prove the exact Ollama store and
llama.cpp GGUF sets operate offline after validation.
