# Phase 10 threat, privilege, and sandbox review

Status: complete on 2026-09-09 within the Phase 10 beta-candidate evidence
boundary. Runtime and support claims remain limited by the exit audit.

## Assets and adversaries

Protected assets are user files, explicit approvals, durable-memory plaintext,
provider inputs and outputs, signing material, update slots, audit integrity,
and the authority represented by privileged services. Relevant adversaries
include malformed local clients, compromised model providers, malicious model
output, tampered packages or updates, confused UI clients, dependency
compromise, and untrusted pull-request code.

## Boundary review

| Surface | Authority and containment | Failure rule | Evidence |
| --- | --- | --- | --- |
| Core tool requests | Closed typed schemas and bounded inputs | Unknown or malformed input is rejected | Rust unit and integration tests; fuzz target |
| Graphical shell | Untrusted presentation client; no policy authority | Digest, request, expiry, or peer mismatch cancels or denies | ADR-0021; shell protocol tests; fuzz target |
| Privileged helper | Fixed operation through system D-Bus and policy | Unknown unit, caller, or operation is denied | ADR-0009/0010; packaging checker |
| Model gateway | Distinct UID, Unix socket, closed runtime files, network-isolated provider | Readiness, peer, audit, or protocol failure refuses service | ADR-0012 through ADR-0020; gateway tests |
| Durable memory | Explicit approved mutation, encrypted store, data-only recall | Missing approval/key/integrity fails closed | ADR-0024/0025; memory tests |
| Distribution/update | Pinned inputs, offline signed metadata, inactive-slot staging | Signature, version, health, or recovery mismatch rejects/rolls back | ADR-0026; lifecycle evidence |
| CI and release | Protected review; untrusted pull-request code does not run in a publication-capable job | Missing SBOM, audit, rebuild match, or provenance blocks candidate | Phase 10 workflows and exit audit |

## Residual risks

- The project has not undergone an external security audit.
- Linux kernel, firmware, compositor, desktop, and distribution packages remain
  a large trusted computing base outside the Rust workspace.
- The VM target does not exercise physical devices, firmware variance, or
  hostile peripheral behavior.
- Fuzzing is bounded evidence, not proof that parsers are defect-free.
- Build provenance establishes origin, not freedom from vulnerabilities.
- Full image byte reproducibility is not yet claimed; release binaries and
  canonical metadata are the Phase 10 practical-reproducibility boundary.

Any new network access, runtime download, telemetry, privileged operation,
hardware claim, or release-support promise requires separate review.
