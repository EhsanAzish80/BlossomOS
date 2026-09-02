# ADR-0001: Open-source license

- Status: Proposed
- Date: 2026-09-02
- Owners: Project maintainers

## Context

Blossom OS is intended to be open source, but the preserved prototype has no
license file. Publishing code without a license does not grant normal open-source
rights. The choice also affects downstream distributions, model/runtime
integrations, contributor expectations, and compatibility with dependencies.

## Decision

Select and add a project license before accepting external contributions or
making the initial public source push. This ADR intentionally does not choose on
the maintainer's behalf.

## Alternatives considered

- Apache-2.0: permissive use plus an explicit patent grant.
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
consent, so decide before accepting them.

## Validation

- Maintainer explicitly approves the license.
- Dependency compatibility is reviewed.
- `LICENSE`, README, package metadata, and contribution guidance agree.
