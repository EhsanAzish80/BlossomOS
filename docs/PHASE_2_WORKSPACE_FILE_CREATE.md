# Phase 2 `files.write:create` evidence

Status: implemented and locally verified on 2026-09-02. Target-Linux evidence is
provided by the Quality workflow.

## Contract

- Tool: `files.write.create`, closed to one selected workspace root, exact
  relative destination, and exact bounded UTF-8 content.
- Capability: `files.write:create`; overwrite is a different, unimplemented
  authority.
- Policy: explicit `ask`; only interactive `Approve once` reaches a write.
- Permissions: fixed `0600`; callers cannot provide modes, flags, temp names,
  mounts, or sandbox configuration.
- Result: durable creation or the distinct non-success state
  `published_durability_uncertain`.

## Containment and publication

The Linux provider selects the absolute workspace root with `openat2` no-symlink
resolution, retains it, and resolves the exact parent with
`RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS |
RESOLVE_NO_XDEV`. Root and parent device/inode identities, destination, content,
digest, and fixed mode are bound into the approval request.

After approval it revalidates retained identities, creates an unnamed inode in
the retained parent with `O_TMPFILE`, enforces `0600`, writes and `fsync`s the
content, reads it back through the same descriptor, and verifies type, bytes,
digest, and mode. Publication links that exact retained inode to the approved
name with descriptor-relative `linkat(AT_EMPTY_PATH)`, followed by
parent-directory `fsync`.

An existing or raced-in destination is never replaced. Pre-publication failures
close the unpublished unnamed inode, leaving no visible cleanup name. A
post-publication directory-sync failure is reported as published but
durability-uncertain and is never automatically retried or deleted.

## Privacy and audit

The interactive preview shows exact root, destination, identities, `0600`, byte
count, digest, and complete JSON-escaped content. Audit omits clear paths,
destination, and content; it stores their digests, identities, transitions,
publication/durability state, and verification.

## Tests

Portable tests cover strict schemas, traversal rejection,
content digest/mode constraints, approval binding/replay, exact preview,
denial/cancellation/expiry/non-TTY behavior, zero executor calls, audit
redaction, durable verification, and truthful durability uncertainty.

Linux-only tests cover production creation, missing parents, root/parent/final
symlinks, destination selection and publication races, no overwrite, retained
parent replacement, fixed permissions, absence of visible temporary names,
atomic publication, directory durability state, and absence of executor calls.

This checkpoint adds no overwrite, append, delete, arbitrary rename, directory
listing, Bash, generic execution, IPC daemon, LLM, privilege, or graphical
shell. The later exact service-status checkpoint is documented separately.
