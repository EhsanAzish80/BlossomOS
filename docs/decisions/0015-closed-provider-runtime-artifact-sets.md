# ADR-0015: Closed provider runtime artifact sets

- Status: Accepted
- Date: 2026-09-03
- Accepted: 2026-09-03 as a prerequisite discovered during the approved Phase 4
  production-package work
- Owners: Project maintainers

## Context

The schema accepted in ADR-0012 measured one provider executable. The pinned
official llama.cpp CPU release is dynamically linked and includes provider
libraries beside `llama-server`. Measuring only the executable while exposing
unmeasured libraries would allow package drift or replacement to change the
code executed inside the provider process. Binding only the executable would
instead make the official package unable to load its bundled dependencies.

## Decision

Provider profile schema version 4 adds an absolute runtime mount, a sorted and
bounded list of every regular file below it, and a SHA-256 digest of the
canonical artifact list. The executable remains explicit and must exactly equal
one entry in that list. The service receives the complete runtime directory as
one read-only bind.

Readiness walks the root-owned runtime directory without accepting symlinks or
special files, rejects unknown or missing entries, then opens, bounds, hashes
and retains every declared file descriptor. Only the declared executable must
have an executable mode. The total runtime bytes and file count are bounded.

Schema version 3 and earlier profiles fail closed. Provider archives remain
untrusted inputs until deterministic packaging has safely extracted them and
produced a canonical manifest that matches the release-compiled registry.

## Consequences

- Dynamically loaded provider code cannot remain outside artifact provenance.
- Adding, removing or changing a provider library requires a reviewed profile
  update.
- Runtime archives with unsafe paths, links, special files or unexpected
  contents cannot be packaged by the future deterministic builder.
- This ADR does not activate a service, install a package or admit private
  input.
