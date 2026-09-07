# Phase 8 inactive encrypted memory store

Status: complete for the inactive encrypted store checkpoint; installed Linux
evidence is tracked in `docs/PHASE_8_INSTALLED_EVIDENCE.md`.

## Boundary

The inactive store uses a versioned binary envelope, a random XChaCha20-Poly1305
nonce, authenticated associated data, and a separately stored 256-bit key.
Data and key directories must be distinct absolute private directories. Store
and key files require mode `0600`; directories require mode `0700`; symlinked
files and directories fail closed.

Updates are written to a randomly named create-only private file, synced,
atomically renamed over the prior encrypted store, and followed by a directory
sync. Failed writes remove their private temporary file. The fixed encrypted
store limit is 1 MiB and individual note values remain limited to 4 KiB.

This checkpoint adds no tool request, engine route, public service, production
path constructor, shell projection, recall, or enabled policy. It cannot be
reached by the model or an ordinary client.

## Cryptographic dependencies

`chacha20poly1305` 0.10.1 is the RustCrypto AEAD implementation used instead of
locally authored cryptography. It is dual MIT/Apache-2.0 licensed, contains no
network or telemetry behavior, and is locked with its transitive dependencies.
`getrandom` 0.3.4 obtains keys, nonces, and temporary-name entropy from the
operating system. `zeroize` 1.9.0 clears in-process key buffers on drop. Both
are likewise lockfile-pinned.

These dependencies expand the unprivileged core's supply-chain surface. They
do not add a process, service, dynamic-code, network, or privilege boundary.
Dependency review, lockfile scanning, and update monitoring remain required.

## Evidence

Deterministic tests prove encrypted round trips, absence of approved plaintext
in the store file, separate key material, authentication failure after
ciphertext or key mutation, rejection of unsafe modes and symlinked
directories, duplicate-record rejection, atomic replacement, and absence of
leftover temporary files after successful replacement.

## Remaining limits

The current key file is only a checkpoint boundary, not a hardware-backed
key design. Backup, recovery, key rotation, secure deletion limits on real
storage media, migration rollback, multi-process locking, and crash/power-loss
fault injection remain later distribution concerns. The public tool registry
stays inactive, so this checkpoint makes no automatic-memory claim.
