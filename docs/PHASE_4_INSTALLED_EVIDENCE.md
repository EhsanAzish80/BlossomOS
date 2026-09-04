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
pinned model is approximately 491 MB. The current authoritative run is
[`33859738402`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/33859738402),
which passed on 2026-09-04 for merged commit `80a59d3`. It verified the installed
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

This run covers the pinned x86-64 llama.cpp profile only. It does not provide a
pinned Ollama package or installed Ollama evidence, target-Arch evidence, or the
terminal-write fault evidence still required by ADR-0018.

Failure diagnostics expose only systemd state/result categories; the workflow
does not publish service journals, model output, prompts or raw gateway audit
records.
