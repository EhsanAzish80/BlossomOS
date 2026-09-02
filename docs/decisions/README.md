# Architecture Decision Records

ADRs record decisions that materially affect architecture, security, privacy,
compatibility, packaging, or contributor obligations.

## Statuses

- Proposed
- Accepted
- Superseded
- Rejected

## Process

1. Copy `0000-template.md` to the next numbered file.
2. Describe context, decision, alternatives, security/privacy consequences, and
   migration or rollback.
3. Review related contract documents and tests.
4. Mark accepted only after explicit project review.
5. Never silently edit the meaning of an accepted ADR; supersede it.

## Initial decisions required

- Open-source license and contributor terms: accepted in ADR-0001.
- Phase 1 implementation language: accepted in ADR-0002.
- Phase 1 Linux sandbox adapter: accepted in ADR-0003.
- Phase 2 capability taxonomy and expansion rules: accepted in ADR-0004.
- Exact-file read containment: accepted in ADR-0005.
- Atomic create-only workspace file writes: ADR-0006 is superseded by the
  unnamed-inode publication correction in accepted ADR-0007.
- Exact systemd service-status reads: accepted in ADR-0008; implementation is
  pending.
- Local IPC transport and schema format.
- Sandbox and resource-control stack.
- Privileged-helper authorization design.
- Audit storage, integrity, redaction, and retention.
- Quickshell/Hyprland support and compatibility policy.
- Model-provider protocol and offline guarantees.
