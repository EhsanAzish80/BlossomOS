# Phase 11 release-candidate audit

Date: 2026-09-16

## Decision

The reviewed source is eligible to produce a new **qualification candidate**.
It is not yet approved as a public or general-purpose Blossom OS release.

The physical installer is intentionally frozen to the qualified Intel Mac
target described in `PHASE_11_INSTALL_GUARD.md`. It must not be represented as a
generic installer for arbitrary computers. The generic VM workflow proves the
separate virtual-disk installation boundary; it does not widen the physical
target policy.

## Source gates completed

- Rust formatting and the complete locked workspace test suite pass.
- The complete Python suite passes, including deterministic manifest/SBOM,
  media re-observation, write verification, installation guards, and Phase 11
  failure cases.
- Every `scripts/ci/check_*.py` fail-closed source boundary passes.
- ShellCheck passes for every changed release/session shell script.
- Actionlint passes for both changed Phase 11 workflows.
- Python source compilation and `git diff --check` pass.
- The desktop launcher is restricted to a fixed action allowlist and rejects
  unknown actions; the QML client cannot choose an executable or arguments.
- The shell UI and recovery services have explicit restart bounds, sandboxing,
  and a terminal recovery path.
- The physical-candidate workflow now executes the complete Python suite and
  every CI boundary checker before building an ISO.

## Candidate gates still required

These gates require the new ISO built from the audited revision and cannot be
closed by static analysis:

1. The pinned Arch build completes and its checksum verifies.
2. The live image reaches the Blossom desktop without manual shell commands.
3. The welcome surface and application launcher render readable text.
4. Files, browser, editor, terminal, network, and audio actions start.
5. The guarded installer is visible only in the live environment.
6. A blank generic x86-64 UEFI VM installs and reboots successfully.
7. The installed VM automatically reaches the desktop and survives one restart.
8. Shell failure opens the bounded recovery terminal instead of leaving an
   unexplained black screen.
9. The frozen Intel Mac target passes live boot, install, disk boot, desktop,
   networking, audio, restart, and shutdown checks.

Any failure in those gates returns the candidate to source review. A successful
ISO build alone is not release approval.

## Release labels

- Before all candidate gates: `qualification candidate`
- After generic VM gates but before frozen hardware gates: `VM-qualified candidate`
- After every gate above: `Phase 11 hardware-qualified candidate`
- `Public release` remains a later decision requiring removal or redesign of
  the frozen single-machine installer boundary and wider hardware validation.
