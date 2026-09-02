# ADR-0001: Open-source license

- Status: Accepted
- Date: 2026-09-02
- Owners: Project maintainers

## Context

Blossom OS is intended to be open source, but the preserved prototype has no
license file. Publishing code without a license does not grant normal open-source
rights. The choice also affects downstream distributions, model/runtime
integrations, contributor expectations, and compatibility with dependencies.

## Decision

Use the Apache License, Version 2.0 (`Apache-2.0`) for Blossom OS. Add the
canonical license text at the repository root before the initial public push.

## Alternatives considered

- Apache-2.0: selected for permissive use plus an explicit patent grant.
- MPL-2.0: file-level copyleft that permits combination with differently licensed
  components.
- GPL-3.0-or-later: strong copyleft for distributed derivative works.

Dependency licenses and the desired downstream policy must be checked before
acceptance.

## Security and privacy consequences

The license does not replace the security policy or warranty disclaimers, but it
defines the legal conditions under which security-sensitive code is modified and
redistributed.

## Operational consequences

CI should check required headers only if the selected license and project policy
need them. Contributor terms and release artifacts must match the decision.

## Migration and rollback

Add `LICENSE`, update contribution and packaging metadata, and record the accepted
SPDX identifier. Relicensing after outside contributions may require contributor
consent.

## Validation

- Maintainer explicitly approved Apache-2.0 on 2026-09-02.
- The root `LICENSE`, README, and contribution guidance identify Apache-2.0.
- No package manifest or release metadata exists yet. Dependency and packaging
  license compatibility must be reviewed when those files are introduced.
