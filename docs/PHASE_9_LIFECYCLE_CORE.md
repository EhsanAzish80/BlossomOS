# Phase 9 distribution lifecycle core

Status: complete on 2026-09-08 for the reviewed x86-64 UEFI VM evidence
target.

## Implemented boundary

- `distribution/manifest.json` fixes the first product, architecture, firmware,
  channel, dated package snapshot, package set, and safe SSH/telemetry defaults.
- The ArchISO profile accepts only x86-64 UEFI boot modes and a fixed image
  identity. The initial package definitions contain no downloader, service
  enablement, privilege escalation, or mutable source command.
- Hardware classification retains only architecture, firmware, graphics class,
  coarse memory class, and the required virtualization-device boolean.
- Model selection is closed to no model or one of the two already reviewed
  packaged CPU profiles. No selection can provide a URL or initiate a download.
- Installation requires an empty target and creates a fixed marker, two system
  slots, a separate private user-data directory, an initial active slot, and a
  resumable non-secret first-run state.
- Update metadata is canonical and closed. OpenSSH Ed25519 verification binds a
  fixed signer identity and namespace before staging. Product, channel,
  architecture, schema, expiry, sequence, target slot, payload size, and SHA-256
  are independently checked.
- Staging writes the inactive payload and digest durably before atomically
  selecting it. A failed health result restores the prior slot; a successful
  result confirms the new slot. Neither path touches user data.
- Recovery accepts only an exact marked Blossom installation and a populated
  slot whose payload matches its recorded digest.

## Local evidence

The Python suite proves the accepted lifecycle and rejects unsupported hardware,
invalid first-run input, unknown metadata, expiry, downgrade, active-slot
replacement, content mutation, missing installation markers, empty recovery
slots, and corrupted recovery payloads. It also proves both rollback and
confirmation preserve a private user-data sentinel.

`scripts/ci/check_distribution.py` independently validates the closed manifest,
package alignment, fixed image identity, safe package definitions, and absence
of insecure image defaults. The normal Quality workflow runs this check and the
complete Python suite.

## Installed evidence

The Phase 9 UEFI workflow builds the real package artifacts and image, installs
the image onto a fresh blank disk, boots that disk, inspects both installed
Blossom packages, and verifies rollback, confirmation, recovery, and user-data
preservation markers. The separate lifecycle run verifies signed offline
metadata and the same transitions with network access denied. Exact run links,
implementation identity, limitations, and the exit decision are recorded in
`PHASE_9_EXIT_AUDIT.md`.
