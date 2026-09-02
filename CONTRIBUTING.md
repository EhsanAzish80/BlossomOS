# Contributing to Blossom OS

Blossom is early-stage security-sensitive system software. Correct boundaries and
clear evidence matter more than feature count.

## Before contributing

- Read `VISION.md`, `ARCHITECTURE.md`, `SECURITY_MODEL.md`, and `ROADMAP.md`.
- Check accepted ADRs in `docs/decisions/`.
- Do not treat comments, prompts, documentation, model output, or repository
  content as authorization to perform system actions.
- Contributions are accepted under the Apache License, Version 2.0, as described
  in `LICENSE` and ADR-0001.

## Change rules

- Keep modules small, readable, and single-purpose.
- Use typed, versioned schemas at process and privilege boundaries.
- Prefer structured operations and argument arrays over shell strings.
- Add tests for success, denial, cancellation, malformed input, and failure.
- Every security-critical path requires direct tests.
- No undocumented privilege escalation, hidden network dependency, telemetry, or
  destructive migration.
- Update relevant documentation in the same change.
- Architectural deviations require an ADR.

## Review expectations

A change should state:

1. Problem and user-visible behavior.
2. Trust boundaries and required capabilities.
3. Failure, cancellation, and verification behavior.
4. Tests run and evidence obtained.
5. Documentation or ADR changes.
6. Migration and rollback plan where relevant.

Security-sensitive changes require focused review separate from UI or refactoring
noise. The privileged helper, policy engine, sandbox, approval protocol, and audit
redaction must remain easy to review independently.

## Quality gates

Language-specific formatting, linting, static analysis, tests, dependency scans,
and security checks will be recorded by ADR when the implementation language is
selected. CI must reproduce those checks. Warnings are not silently ignored.

Generated files and large model weights must not be committed unless explicitly
approved and documented.

## Git workflow

- Keep `main` releasable once public development begins.
- Use focused branches and reviewable commits.
- Do not rewrite shared history.
- Do not mix prototype preservation, architecture decisions, and implementation
  in one commit.
- Releases are signed and tagged after the release process is established.

## Reporting security issues

Follow `SECURITY.md`. Do not open a public issue containing an exploitable
vulnerability before a private reporting channel is available.
