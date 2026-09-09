# Blossom OS documentation

This directory contains the reviewed engineering record for Blossom OS.
Documents describe either implemented evidence, an accepted design boundary, or
explicitly labeled future work. A target design is not an implementation claim.

## Start here

| Document | Purpose |
| --- | --- |
| [Vision](../VISION.md) | Product purpose and principles |
| [Architecture](../ARCHITECTURE.md) | Target components and trust boundaries |
| [Security model](../SECURITY_MODEL.md) | Assets, threats, controls, and required tests |
| [Roadmap](../ROADMAP.md) | Ordered phases and exit gates |
| [Contributing](../CONTRIBUTING.md) | Change, review, evidence, and Git expectations |
| [Security policy](../SECURITY.md) | Private vulnerability reporting and support status |
| [Architecture decisions](decisions/README.md) | Accepted and superseded ADRs |

## Phase evidence

| Phase | Reviewed boundary | Primary record |
| --- | --- | --- |
| 0 | Preserved prototype and repository baseline | [Baseline](PHASE_0_BASELINE.md) |
| 1 | Deterministic security vertical slice | [Security core](PHASE_1_SECURITY_CORE.md) |
| 2 | Typed capability and sandbox foundation | [Exit matrix](PHASE_2_BASELINE.md) |
| 3 | One fixed privileged operation | [Exit baseline](PHASE_3_BASELINE.md) |
| 4 | Replaceable local model runtime | [Exit audit](PHASE_4_EXIT_AUDIT.md) |
| 5 | Closed planning and truthful outcomes | [Exit audit](PHASE_5_EXIT_AUDIT.md) |
| 6 | Narrow Blossom Shell approval surface | [Exit audit](PHASE_6_EXIT_AUDIT.md) |
| 7 | Battery and coarse network context | [Exit audit](PHASE_7_EXIT_AUDIT.md) |
| 8 | Explicit encrypted durable notes | [Exit audit](PHASE_8_EXIT_AUDIT.md) |
| 9 | Distribution, signed updates, and rollback | [Exit audit](PHASE_9_EXIT_AUDIT.md) · [Baseline](PHASE_9_BASELINE.md) · [Lifecycle core](PHASE_9_LIFECYCLE_CORE.md) |
| 10 | Public-beta hardening, active and not yet complete | [Baseline](PHASE_10_BASELINE.md) · [Threat review](PHASE_10_THREAT_REVIEW.md) · [Limitations](PHASE_10_LIMITATIONS.md) · [Checklist](PHASE_10_RELEASE_CHECKLIST.md) |

Each phase may have additional design, implementation, and installed-evidence
records beside its primary document. Filenames use `PHASE_<number>_...` so the
full record remains discoverable without duplicating it here.

## Engineering policy

- [Branch and release policy](BRANCH_RELEASE_POLICY.md)
- [Dependency policy](DEPENDENCY_POLICY.md)

## Historical prototype guides

[QUICK_START.md](QUICK_START.md) and [SIMPLE_GUIDE.md](../SIMPLE_GUIDE.md)
describe the preserved early prototype. They are retained for provenance only;
they are not supported installation instructions. Phase 9 established the first
reviewed distribution, installation, update, recovery, and rollback path for an
evidence-only x86-64 UEFI VM target; it is not a supported public installer.

## Document status convention

- **Accepted**: a reviewed design decision; implementation may still be pending.
- **Complete**: the document's exact checkpoint passed its stated evidence.
- **Active**: work is underway and must not be presented as complete.
- **Deferred**: intentionally outside the current reviewed boundary.
- **Historical**: preserved provenance, not current operational guidance.

Completion is always scoped. A passing deterministic test is not installed-host
evidence, and installed-host evidence is not a supported release claim.
