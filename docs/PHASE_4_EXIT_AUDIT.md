# Phase 4 exit audit

Status: complete on 2026-09-04. This is a pre-alpha engineering checkpoint,
not a supported runtime, distribution, installer, hardware, or release claim.

## Accepted boundary

- ADR-0011 through ADR-0019 are accepted. Together they define provider-neutral
  requests, untrusted model output, closed provider/model artifact sets, fixed
  installed identities, authenticated private admission, cancellation,
  content-free operational audit, two pinned CPU-only x86-64 packages, and
  root-owned active-profile selection.
- Private inference frames carry no provider, model, endpoint, path,
  classification, runtime, mount, capability, policy, approval, or execution
  authority. The admitted code-owned profile supplies provider identity.
- Model tool output is only a schema-validated proposal. It cannot approve or
  execute a capability and cannot bypass the broker, policy, approval,
  verification, or audit boundaries.
- The production listener is compiled only by the non-default
  `production-private-inference` feature. Default builds open no private socket.

## Deterministic evidence

- Both Ollama and llama.cpp adapters have closed, bounded conformance tests for
  requests, streaming, cancellation, malformed output, proposed intents,
  deadlines, endpoint restrictions, and terminal sequencing.
- Linux tests cover Unix peer credentials, retained installed-account evidence,
  one-request framing, pipelining rejection, descriptor-bound artifacts,
  symlink rejection, readiness, audit ordering, capacity failure, and socket
  lifecycle.
- Repository checks verify canonical registry/profile bytes, deterministic
  package roots, rendered units, pins, digests, sizes, and template drift.

## Installed real-model evidence

- llama.cpp: workflow run
  [`33865392657`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/33865392657)
  passed for merged main. The earlier run `33860951348` also records the
  terminal-write containment addition at commit `91830c3`.
- Ollama: workflow run
  [`33866069049`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/33866069049)
  passed for merged main.
- Each disposable Ubuntu x86-64 run installed one exact package and exercised
  real offline inference, distinct non-login identities, peer admission,
  private network namespace, provider external-network denial, filesystem
  denial/read-only mounts, bounded resources, cancellation races, provider
  loss, audit ownership/redaction/capacity, terminal-write containment and
  recovery, orderly cleanup, and stale-socket refusal.
- The providers receive no private input over unauthenticated loopback. The
  authenticated Unix gateway remains the sole private-input ingress.

## Target-Arch evidence

Workflow run
[`33867077966`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/33867077966)
passed in a digest-pinned official Arch Linux x86-64 userspace. It ran the model
runtime and gateway tests as a non-root account, ran strict workspace clippy,
built the feature-gated gateway, assembled both closed packages from immutable
inputs, verified their systemd units and ELF dependencies, executed both
provider binaries, and confirmed gateway startup fails closed without valid
installed readiness and creates no socket.

This is Arch userspace, package-construction, and ABI evidence on a GitHub-hosted
Linux kernel. It is not evidence for an Arch kernel, ArchISO boot, installer,
desktop session, GPU, physical hardware, package repository, upgrade, rollback,
or signed release.

## Exit decision

The Phase 4 exit criterion is satisfied for the two explicitly supported pinned
x86-64 CPU evidence profiles. Deterministic conformance passes for both;
real-model installed operation is verified with provider external networking
disabled; the authenticated gateway prevents a provider path from becoming an
authorization or execution path.

Private input remains disabled in default builds. Adding an architecture,
provider, model, mutable downloader, acceleration backend, or user-facing model
lifecycle requires a new reviewed expansion and equivalent evidence. Phase 5
may build orchestration but must continue treating all model and tool content as
untrusted and must never infer success from dispatch alone.
