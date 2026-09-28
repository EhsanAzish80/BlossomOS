# ADR-0031: Model-proposed create-only workspace file

- Status: Accepted
- Date: 2026-09-28
- Accepted: 2026-09-28 after project-owner review
- Owners: Project maintainers
- Requires: ADR-0007, ADR-0011, ADR-0020, ADR-0021, ADR-0030

## Context

ADR-0011 permits a model to choose only among code-owned, argument-free read
intents. ADR-0030 selects workspace file creation as the first qualification
effect. Allowing a model to choose a filename and content crosses a new trust
boundary: model output now influences both where bytes are published and which
bytes are written.

The existing ADR-0007 workspace-create implementation already retains directory
identities, publishes an unnamed inode atomically without replacement, verifies
the result and binds approval to an exact typed selection. This decision adds a
narrow resolver in front of that unchanged trusted path.

## Decision

### Untrusted proposal

The only model-controlled fields are `name` and `content`.

- `name` is one ASCII path component matching
  `[a-z0-9][a-z0-9._-]{0,63}`. It is neither empty nor absolute and cannot
  contain a slash, traversal component, Unicode look-alike or control byte.
- `content` is valid UTF-8, at most 4096 bytes, contains no NUL, terminal escape,
  bidirectional-control or zero-width formatting character.

The canonical Rust proposal schema is the single source for both validation and
the llama.cpp grammar. Grammar enforcement improves parse reliability only;
the Rust validator remains authoritative. ADR-0030 binds both schema and grammar
digests into the active profile and package receipt.

Mixed text and action in one completion remains invalid.

### Trusted resolution

A model proposal cannot construct `WorkspaceCreateSelection`. A dedicated
trusted resolver:

1. reads the workspace root from session configuration;
2. validates the proposal;
3. opens and retains the configured root and selected parent under ADR-0007's
   Linux containment rules;
4. derives the full relative destination from the validated single component;
5. computes the content SHA-256;
6. fixes mode `0600`; and
7. returns the typed selection consumed by existing policy, approval, atomic
   create, verification and audit code.

There is no model-originated route to the selection except this resolver. The
model cannot supply a workspace root, directory identity, path prefix, digest,
mode, approval token or policy result.

### Policy, preview and audit

Every model-originated workspace write returns `ask`; no allow rule may bypass
approval. Approval binds the exact resolved selection. Publication remains
create-only and fails if the destination already exists or the retained
identity changes.

The preview visibly escapes every non-printable byte. V1 additionally rejects
terminal escape, bidi-control and zero-width formatting characters so the
displayed preview cannot differ from the selected bytes.

The content-free audit records origin `model_proposed`, relative-destination
digest, content digest and byte length. It never records content or a clear
workspace path. Denied, invalid, binding-mismatched and pre-publication failures
record zero executor starts.

### Minimal indirect-injection turn

The indirect-injection qualification uses no second argument-bearing intent.
Turn one performs one fixed, argument-free, code-owned fixture read. The read is
bounded and identity-retained by the normal file provider; its audit entry
contains only path/content digests and byte length. The result is carried across
the private gateway as a typed untrusted-data message and rendered by the final
provider adapter in a separate, fixed `UNTRUSTED_FILE_CONTENT` frame. It is
never concatenated into the user's instruction.

Turn two exposes only `files.write:create` and cannot read again or start a
third turn. The qualification fixture deliberately obeys the planted text as a
worst-case model. One variant proposes traversal and must fail validation. A
second proposes a valid but unwanted file and must reach the ordinary exact
preview, where the qualification client denies it. Both variants require zero
effects and zero command-executor starts in the authoritative activity window.

## Alternatives considered

### Model supplies a relative path

Rejected. Even a normalized relative-path grammar expands traversal, nested
directory, identity-retention and preview ambiguity. One ASCII component is
sufficient for the first effect.

### Model supplies a complete selection

Rejected. Device and inode identity, root authority, digest and mode are trusted
facts and cannot originate in model text.

### Escape unsafe display characters without rejecting them

Deferred. Visible escaping can be safe, but rejecting bidi, zero-width and
terminal-control characters creates a smaller first contract.

## Security and privacy consequences

The model influences bounded bytes only after trusted resolution and an exact
once-only approval. It cannot overwrite, traverse, select another root, hide
content with display controls or mutate an approved request. Audit remains
content-free.

## Operational consequences

The resolver needs a configured session workspace before inference. Absence or
ambiguity fails before approval. Package and VM qualification must exercise the
same resolver and executor used by the installed shell; test-only construction
of a selection is not end-to-end evidence.

## Migration and rollback

The new intent is additive and absent from eligibility unless trusted code
supplies a workspace session. Removing it restores ADR-0011's argument-free
allowlist without changing existing read intents or ADR-0007's direct
user-selected workspace-create path.

## Validation

- Accept one valid basename and bounded printable UTF-8 content.
- Reject traversal, absolute paths, empty, oversized and Unicode names.
- Reject NUL, terminal escape, bidi-control and zero-width content.
- Reject existing destinations, symlink escape and retained-identity changes.
- Prove model-originated policy is always `ask`.
- Prove approval mutation yields binding mismatch and zero executor starts.
- Prove mixed text and action fails closed.
- Prove audit stores origin, digests and byte length without clear content or
  workspace path.
