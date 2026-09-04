# Phase 4 installed-service evidence

Status: passing llama.cpp and Ollama installed-system evidence plus pinned Arch
userspace/package/ABI evidence recorded. The harnesses are manually dispatched.

`.github/workflows/phase4-installed-evidence.yml` uses a disposable x86-64
Ubuntu runner. It builds the non-default production gateway, downloads only the
immutable URLs in the reviewed lock, and relies on the package builder to reject
every digest or size mismatch before installation.

The harness creates the real static service identities plus separate authorized
and unlisted probe accounts, installs the generated package tree, and starts the
actual namespace, llama.cpp and gateway units under systemd. Its content-free
probe checks root, unlisted-user and provider rejection, authorized private
inference, request-bound streaming cancellation, exact socket metadata, distinct
identities, shared private network namespace, external-network denial, selected
systemd resource limits, audit ownership/redaction, and provider-loss
non-success.

The workflow intentionally does not run for every pull request because the
pinned model is approximately 491 MB. The current authoritative run is
[`33860951348`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/33860951348),
which passed on 2026-09-04 for merged commit `91830c3`. It verified the installed
llama.cpp boundary end to end: package assembly and installation, distinct
service identities, socket metadata and admission denial, a shared private
network namespace with external-network denial, selected resource limits, real
offline inference, request-bound cancellation after a validated start, a
content-free cancelled terminal audit record, audit
ownership/isolation/redaction and provider-loss non-success. The gateway and
provider identities could not observe fixed host-only home, optional,
configuration, process and cross-service paths in their actual mount
namespaces, and their code-owned binary/model mounts were read-only. The run
also verified orderly socket cleanup followed by application-level refusal of a
stale regular file injected immediately before the packaged gateway executable
in the same service sandbox. From a fresh one-record journal, at least 1,000
gateway-level root rejections then exhausted the closed audit capacity within a
5,000-attempt bound. The gateway failed with `Restart=no`, removed its socket
and non-preserved boot-scoped journal, and left the provider and namespace
services active.

The same run used a fixed, content-free protocol fixture outside the production
package and signed provider inventory to exercise cancellation while a provider
connection was deliberately stalled, while response headers were withheld, and
after the first validated text delta but before completion. In every case the
request-bound cancellation produced `Cancelled`, never a successful completion
or proposed tool intent. The fixture accepts only one bounded HTTP request on
the fixed loopback endpoint, uses the pinned logical model identity, logs no
request or response content, and runs only on the disposable evidence host.
The client then retained its request half but refused further reads after a
validated text delta. The provider completed after that refusal; the gateway
synced the completed provider outcome before its terminal write failed, stayed
active, and served a subsequent real inference successfully. This proves local
terminal-write containment, not that a peer consumed any successfully written
bytes.

## Ollama installed evidence

`.github/workflows/phase4-ollama-installed-evidence.yml` applies the same
installed boundary to the separately pinned Ollama v0.33.3 CPU x86-64 runtime
and exact Qwen 2.5 0.5B manifest/blob set. Authoritative run
[`33866069049`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/33866069049)
passed package assembly, installed services, identity and namespace checks,
external-network and filesystem denials, real offline inference, cancellation,
provider races, terminal-write containment, audit redaction/capacity/provider
loss, cleanup, and stale-socket refusal.

Only a root-owned canonical `active.json` may select one of the two compiled-in
profiles. The private request cannot select or fall back between providers.

## Pinned Arch userspace evidence

`.github/workflows/phase4-arch-abi-evidence.yml` uses the digest-pinned official
Arch Linux x86-64 container. Authoritative run
[`33867077966`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/33867077966)
passed non-root Rust tests, strict workspace clippy, feature-gated gateway build,
both deterministic package assemblies, systemd unit verification, ELF
dependency inspection, provider binary execution, and fail-closed gateway
startup with no socket when installed readiness is absent.

The Arch workflow shares the GitHub-hosted Linux kernel. It does not prove an
Arch kernel, ArchISO, installer, desktop session, GPU, physical hardware,
package repository, upgrade, rollback, or signed release.

Failure diagnostics expose only bounded state/result categories; the workflows
do not publish service journals, model output, prompts or raw gateway audit
records.
