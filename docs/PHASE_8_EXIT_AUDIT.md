# Phase 8 exit audit

Status: complete on 2026-09-07.

## Passing boundary

- ADR-0024 fixes five non-interchangeable memory classes and permits only an
  explicit user-authored durable note. ADR-0025 permits only bounded,
  chronological, data-only recall.
- The service begins disabled. Existing tool and shell request registries do
  not expose memory, so neither a model nor an ordinary existing client can
  activate or write it invisibly.
- Create, edit, and delete require policy `Ask`, an exact digest-bound preview,
  a fresh once-only approval, and post-write verification. An `Allow` rule
  cannot bypass these mutation approvals.
- Denial, cancellation, expiry, replay, service replacement, request mutation,
  stale edits, corruption, unsafe permissions, symlinks, missing keys, and
  verification failures fail closed.
- The XChaCha20-Poly1305 store has a versioned authenticated envelope, separated
  key directory, private modes, bounded records, random nonces, atomic synced
  replacement, and no plaintext or command fallback.
- Inspect, edit, delete, export, disable, until-deleted retention, and bounded
  recall are fixed operations. Recall is explicitly data, never permission,
  approval, identity proof, policy, or present-state verification.
- The shell-facing projection is deliberately narrow and contains no path,
  key, cipher, database, query, filesystem, command, or generic memory
  authority.
- Mutation audit is content-free and hash-chained. Tests prove that private
  values do not serialize into it and that event mutation breaks verification.

## Evidence

- Signed implementation commit `d0ee06676c54ce5cd6ea613609118d8521ed617e`
  has a valid maintainer ED25519 signature.
- Pull request 137 passed exact-head repository/package tests, CodeQL,
  dependency review, and secret scan before merge commit
  `a5f94eb44d33ef21bab12b69041dab95a52783fc`.
- Local strict formatting, Clippy with warnings denied, repository checks, and
  the complete Rust workspace suite passed. The core suite contains 233 tests,
  including six focused service tests and the encrypted-store adversarial
  cases.
- Installed x86-64 Linux workflow run
  [`34134127972`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34134127972)
  passed the real encrypted create, restart, inspect, edit, recall, export,
  delete, audit, disable, permissions, and plaintext-absence path.

## Limits preserved

This phase completes the accepted bounded memory slice, not every possible
personalization feature. There is no automatic capture, transcript storage,
model-authored memory, temporary-memory service, project-knowledge import,
semantic search, embeddings, cross-class query, sync, sharing, backup, key
recovery, hardware-backed key custody, secure-erasure guarantee, or production
public activation. Version 1 has no predecessor migration; unsupported future
schemas fail closed and require a separately tested atomic migration.

Distribution packaging, installer integration, upgrades, rollback, broader
hardware support, and recovery remain Phase 9 work.

## Decision

Phase 8 is complete at the reviewed explicit durable-note boundary. Every
durable mutation is visible and exactly approved, every lifecycle control has a
truthful bounded result, installed evidence passes, and no durable memory can
be created invisibly.
