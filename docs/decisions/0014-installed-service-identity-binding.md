# ADR-0014: Bind installed service identities by fixed names

- Status: Accepted
- Date: 2026-09-03
- Accepted: 2026-09-03 as a corrective prerequisite to the approved Phase 4
  production-package work
- Owners: Project maintainers

## Context

ADR-0012 requires persistent, named, non-login service accounts created through
`systemd-sysusers` without globally collision-prone numeric IDs. Provider
profile schema version 2 nevertheless serialized expected numeric UIDs and
GIDs. A release-compiled registry cannot know IDs that the target system assigns
only when the package is installed. Substituting them after installation would
also break the requirement that an installed manifest exactly match its
code-owned canonical registry entry.

## Decision

Provider profile schema version 3 serializes only these code-owned names:

- `blossom-model-gateway` user and primary group;
- `blossom-model-provider` user and primary group; and
- `blossom-ai` access group.

The names are closed constants rather than package or caller inputs. Readiness
opens the root-owned account databases once, resolves those names to numeric
IDs, and rejects missing, duplicate, root, shared, login-capable or incorrectly
grouped identities. The resolved numeric IDs are retained in readiness evidence
and are the only values used for connected-peer and artifact-ownership checks.
They are not written back into the manifest.

Schema version 2 and profiles containing numeric identity fields are rejected.
There is no compatibility fallback or caller-selected account mapping.

This decision corrects only the manifest representation in ADR-0012. Its trust
split, fixed accounts, peer checks, packaging ownership and fail-closed runtime
requirements remain unchanged.

## Consequences

- One release registry can be valid across systems with different allocated
  system-account IDs.
- Package installation cannot silently rewrite the canonical profile.
- Readiness must succeed before any numeric identity is trusted or any private
  input is admitted.
- Changing an account name requires a new profile schema and review.
