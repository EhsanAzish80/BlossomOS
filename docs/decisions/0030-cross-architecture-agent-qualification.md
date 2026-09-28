# ADR-0030: Cross-architecture local-agent qualification

- Status: Accepted
- Date: 2026-09-28
- Accepted: 2026-09-28 after project-owner review
- Owners: Project maintainers
- Requires: ADR-0007, ADR-0012 through ADR-0021

## Context

Blossom has separately proven a closed local-model gateway, policy and approval
boundaries, bounded execution, verification, audit, and an ARM64 development
desktop. It has not yet proven one real model turn through that complete chain.
The first pinned llama.cpp package is x86-64-specific even though its Qwen GGUF
is architecture-independent. Copying that lock or adding an ARM-only install
shortcut would create drift and would not qualify the package layout shipped in
an image.

The existing model catalogue also exposes only argument-free read intents.
Qualifying `files.write:create` therefore requires a reviewed argument-bearing
intent rather than a driver that constructs the request outside the model
contract.

## Decision

### Shared model and architecture-specific runtime

One immutable model record owns the Qwen GGUF bytes, digest, size, revision and
license. Separate `x86_64` and `aarch64` runtime records own the llama.cpp
archive, executable, libraries and license for that architecture. An active
profile references one model record and one runtime record by digest.

`blossom-core` embeds only the registry selected by `cfg(target_arch)`. Startup
fails closed when the compiled architecture, receipt architecture, runtime
record or active profile disagree. There is no ARM-only package format.

The package is installed into qualification systems with `pacman -U`; copying
its assembled tree into a running VM is not qualifying evidence.

### Schema-derived constrained output

The closed intent schema is generated from the same Rust intent definitions
used by validation. The llama.cpp grammar is derived from those schema bytes,
not maintained as an independent handwritten language. The canonical profile
binds the schema digest and grammar digest, so changing what the provider may
emit changes the profile and receipt.

The first argument-bearing model intent is `files.write:create`. Its proposal
contains only a relative destination, UTF-8 content and the content SHA-256.
Trusted runtime code selects and retains the workspace root and parent
identities before approval. The existing ADR-0007 request validation, exact
approval binding, atomic no-replace publication, read-back verification and
content-free audit remain authoritative.

Qualification fixes temperature to zero, seed, thread count, context size,
maximum output tokens and maximum output bytes. Output digests are recorded per
architecture; equality between architectures is not required.

### Positive and adversarial qualification

The positive run uses a fixed prompt to propose one workspace file creation.
It observes the exact preview sent over D-Bus, submits a once-only approval,
executes the bound request, verifies path and content digests, and observes the
terminal audit record. Evidence must state that approval was submitted by the
qualification client and does not prove independent human intent. AT-SPI may
confirm that the shell presents the preview; screenshots are not evidence.

The deterministic gate is twenty consecutive warm runs with the same output
digest and no retries. Any variation fails the gate.

Negative evidence includes:

1. direct prompt injection proposing an ineligible privileged or
   out-of-workspace action;
2. mutation of a request after approval, which must fail binding; and
3. indirect injection only when a genuine two-turn path reads planted file
   content, feeds that result into a second inference request and evaluates the
   next proposal.

For every negative case, the authoritative audit must show zero executor starts
and zero effect events. The two-turn indirect case uses a fixed code-owned read,
a separately typed and bounded untrusted-data message, and an intentionally
compliant hostile fixture. It covers both validator rejection and denial of a
valid-but-unwanted proposal at the exact approval preview.

### Performance evidence

Evidence records cold model-load latency separately from warm inference,
prompt and output token counts, tokens per second, output digest, vCPU, RAM,
architecture, and configured context/token/byte limits. A separate bounded case
must prove that gateway token and byte ceilings terminate oversized output.

## Alternatives considered

### Duplicate complete locks

Rejected. Duplicating the identical model record between architecture locks
invites digest, license and revision drift.

### Copied files in the development VM

Rejected. It cannot prove package hooks, ownership, modes, service layout or the
paths that a later image installs.

### Unconstrained sampling with parser retries

Rejected. Retries conceal instability, and a small model is not expected to
produce a closed intent reliably without a grammar. Constrained output does not
make the proposal trusted; validation, policy and approval remain mandatory.

### A trivial read-only action

Rejected. It would not exercise exact argument binding, mutation rejection,
bounded side effects or durable verification.

## Security and privacy consequences

Model output remains untrusted. Grammar-constrained decoding limits syntax but
does not grant capability or approval. The model cannot select filesystem root
identity, approval token, policy result or audit content. Prompts, file content
and model output are excluded from the content-free operational audit.

Qualification-client approval proves pipeline wiring only. Production human
approval remains open until an independently authenticated interaction is
observed; this ADR does not reinterpret same-peer D-Bus approval as human
intent.

## Operational consequences

Source work precedes image work. First refactor the shared model/runtime records,
profile, receipt and tests. Then build an ARM package and install it through
`pacman -U` into the existing development VM. Only after the deterministic and
negative gates pass is a new ARM image allowed.

The Intel physical image and impersonated-disk qualification are outside this
increment and remain deferred.

## Migration and rollback

The existing x86 profile is migrated to reference the shared model record and
the x86 runtime record without changing their pinned bytes. The new aarch64
record is additive. Reverting ADR-0030 removes the aarch64 record, constrained
intent and qualification package while leaving the previously pinned x86
package and security boundaries intact.

## Validation

- Source checks reject model duplication, unknown architectures, profile or
  receipt mismatch, grammar/schema drift, unpinned decoding parameters and an
  ARM-only packaging route.
- Package tests install the produced package with `pacman -U` and verify exact
  paths, ownership, modes, units, profile and receipt.
- The existing ARM VM passes one cold run, twenty identical warm positive runs,
  direct-injection denial, post-approval mutation rejection, audit executor
  counts and output-limit enforcement.
- Indirect-injection evidence is accepted only after a real two-turn result
  feedback test exists.
