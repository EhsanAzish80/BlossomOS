# Phase 9 distribution and updates baseline

Status: active from 2026-09-08.

Phase 9 begins from the completed Phase 8 security and memory boundaries. Its
goal is a repeatable Blossom-owned x86-64 VM installation and a signed,
fail-closed update and rollback lifecycle.

## Ordered checkpoints

1. Accept ADR-0026 and freeze the first distribution, installer, update, and
   rollback boundary. Complete.
2. Replace prototype distribution inputs with closed Arch package definitions
   for the reviewed Blossom binaries, services, shell, policy, and profiles.
   Core definitions and static validation are implemented; package-build and VM
   installation evidence remain pending.
3. Add a Blossom-owned ArchISO profile with fixed package snapshot metadata,
   branding, no insecure defaults, and deterministic repository validation.
4. Add minimal local hardware classification and closed packaged model
   selection. The default remains no active model. Core state and adversarial
   validation are complete in `docs/PHASE_9_LIFECYCLE_CORE.md`.
5. Add the resumable first-run state machine and installation marker without
   recording secrets. Core state and adversarial validation are complete.
6. Add canonical Ed25519-signed offline update metadata, strict verification,
   inactive-slot staging, atomic boot selection, health confirmation, and
   automatic failed-boot rollback. The local lifecycle is complete; real boot
   integration remains pending.
7. Add installation-media recovery that rejects unmarked targets and can only
   restore a previously verified slot. The local lifecycle is complete; media
   integration remains pending.
8. Prove adversarial package, image, first-run, update, interruption, downgrade,
   signature, recovery, and user-data preservation behavior locally.
9. Build the ISO and prove fresh UEFI VM install, disk boot, failed-update
   rollback, and confirmed update on the trusted x86-64 Linux runner.
10. Perform an independent Phase 9 exit audit and update the roadmap only after
    every gate passes.

## Evidence flow

```text
reviewed source and pinned Arch snapshot
  -> digest-bound Blossom packages
  -> Blossom-owned ArchISO
  -> blank x86-64 UEFI VM disk
  -> marked installation and explicit first-run
  -> signed offline update staged to inactive slot
  -> verified boot health or automatic prior-slot rollback
  -> content-minimized evidence record
```

## Non-goals

This phase does not claim physical-device support, other architectures, legacy
BIOS, Secure Boot, disk encryption, dual boot, migration from prototype images,
network updates, unattended updates, public signing-key readiness, reproducible
build equivalence across hosts, or a supported public release.

## Exit evidence

- Accepted ADR-0026 and closed distribution schemas.
- Reviewed package and image definitions with pinned inputs and checksums.
- No development credentials, autologin, enabled SSH, telemetry, or mutable
  runtime download.
- Strict hardware, model, first-run, installer, update, and recovery state
  machines with adversarial tests.
- Signed offline update proof and interruption-safe A/B rollback.
- Fresh install and both rollback and confirmation paths in a disposable UEFI
  x86-64 VM.
- Protected checks, signed implementation commits, and independent exit audit.

Completion remains scoped to the evidence target. Phase 10 owns public-beta
hardening, broader hardware, release operations, and support claims.
