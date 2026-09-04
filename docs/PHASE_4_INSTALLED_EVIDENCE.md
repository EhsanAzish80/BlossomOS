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
inference, request-bound streaming cancellation, exact socket metadata, distinct
identities, shared private network namespace, external-network denial, selected
systemd resource limits, audit ownership/redaction, and provider-loss
non-success.

The workflow intentionally does not run for every pull request because the
pinned model is approximately 491 MB. Run
[`33853095060`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/33853095060)
passed on 2026-09-04 for merged commit `ed0d8c1`. It verified the installed
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
in the same service sandbox.

This run covers the pinned x86-64 llama.cpp profile only. It does not provide a
pinned Ollama package or installed Ollama evidence, and it does not yet cover
connect/header/completion cancellation races or audit-capacity faults required
by ADR-0017/0018.

Failure diagnostics expose only systemd state/result categories; the workflow
does not publish service journals, model output, prompts or raw gateway audit
records.
