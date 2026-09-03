# Phase 4 installed-service evidence

Status: harness implemented but not yet successfully executed. It is manually
dispatched and does not complete any Phase 4 exit criterion by existing alone.

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
pinned model is approximately 491 MB. A passing manually dispatched run and its
immutable commit SHA must be recorded here before any evidence item is marked
satisfied. It does not yet cover every cancellation race, audit-capacity fault,
stale socket recovery or all filesystem denials required by ADR-0017/0018.

Failure diagnostics expose only systemd state/result categories; the workflow
does not publish service journals, model output, prompts or raw gateway audit
records.
