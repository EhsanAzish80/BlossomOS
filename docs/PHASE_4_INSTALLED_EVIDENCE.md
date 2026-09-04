# Phase 4 installed-service evidence

Status: passing llama.cpp installed-system evidence recorded. The harness is
manually dispatched; this evidence does not by itself complete Phase 4.

`.github/workflows/phase4-installed-evidence.yml` uses a disposable x86-64
Ubuntu runner. It builds the non-default production gateway, downloads only the
immutable URLs in the reviewed lock, and relies on the package builder to reject
every digest or size mismatch before installation.

The harness creates the real static service identities plus separate authorized
and unlisted probe accounts, installs the generated package tree, and starts the
actual namespace, llama.cpp and gateway units under systemd. Its content-free
probe checks root, unlisted-user and provider rejection, authorized private
inference, exact socket metadata, distinct identities, shared private network
namespace, external-network denial, selected systemd resource limits, audit
ownership/redaction, and provider-loss non-success.

The workflow intentionally does not run for every pull request because the
pinned model is approximately 491 MB. Run
[`33847719169`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/33847719169)
passed on 2026-09-04 for merged commit `bba9aea`. It verified the installed
llama.cpp boundary end to end: package assembly and installation, distinct
service identities, socket metadata and admission denial, a shared private
network namespace with external-network denial, selected resource limits, real
offline inference, audit ownership/isolation/redaction and provider-loss
non-success.

This run covers the pinned x86-64 llama.cpp profile only. It does not provide a
pinned Ollama package or installed Ollama evidence, and it does not yet cover
every cancellation race, audit-capacity fault, stale-socket recovery or all
filesystem denials required by ADR-0017/0018.

Failure diagnostics expose only systemd state/result categories; the workflow
does not publish service journals, model output, prompts or raw gateway audit
records.
