# ADR-0006: Atomic create-only workspace file writes

- Status: Superseded by ADR-0007
- Date: 2026-09-02
- Owners: Project maintainers

## Context

> ADR-0007 supersedes the named-temporary-file publication mechanism below
> after implementation review identified a source-name replacement race.

ADR-0004 requires the first workspace write to bind authority to one approved
workspace root and one exact relative destination. It must address traversal,
symlinks, mount changes, replacement races, creation, overwrite, permissions,
durability, backup, and rollback before write access is enabled.

Combining creation and overwrite would introduce materially different failure
and recovery semantics. An overwrite needs ownership policy, backup retention,
conflict detection, and a user-visible recovery path. Those are not prerequisites
for a useful first operation that creates one previously absent text file.

## Decision

Implement one Linux-only `files.write:create` tool. It atomically creates one
new bounded UTF-8 file beneath one user-selected workspace root. Existing
destinations are never replaced. There is no generic write API.

### Workspace and destination selection

The CLI accepts:

1. one exact absolute workspace-root path;
2. one exact relative destination path; and
3. UTF-8 content of at most 64 KiB.

The root follows ADR-0005 absolute-path validation. It is opened before approval
from `/` with Linux `openat2`, `O_DIRECTORY | O_CLOEXEC`, and
`RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS`. The retained
descriptor must identify a directory.

The destination rejects absolute paths, empty or repeated components, `.` and
`..`, trailing separators, NUL/control characters, and oversized input. Its
final component is a filename; the preceding parent path is resolved from the
retained root descriptor with `openat2`, `O_DIRECTORY | O_CLOEXEC`, and:

- `RESOLVE_BENEATH`;
- `RESOLVE_NO_SYMLINKS`;
- `RESOLVE_NO_MAGICLINKS`; and
- `RESOLVE_NO_XDEV`.

The retained parent descriptor must remain on the selected root filesystem.
Workspace and parent device/inode identities, exact relative destination,
content bytes, content digest, and requested mode form the typed request and
approval scope. The requested mode is fixed in code to `0600`; callers cannot
choose it.

Selection verifies through descriptor-relative lookup that the destination is
absent. This is advisory for the preview; execution still relies on atomic
no-replace primitives. Selection opens and stats directories but performs no
write. Denial, cancellation, expiry, or non-TTY invocation closes descriptors
and creates nothing.

### Atomic creation and verification

After once-only approval, the provider:

1. revalidates retained root and parent identities;
2. creates a hidden temporary regular file in the retained parent with
   `O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC`, using a bounded code-generated
   name and mode `0600`;
3. applies exact `0600` permissions with the retained file descriptor;
4. writes the approved bytes through that descriptor with no shell or executor;
5. flushes file data and metadata with `fsync`;
6. verifies type, identity, length, permissions, and SHA-256 from the temporary
   descriptor;
7. publishes with descriptor-relative `renameat2(..., RENAME_NOREPLACE)` to the
   exact final filename; and
8. `fsync`s the retained parent directory before reporting durable success.

Temporary-name collisions retry a small fixed number of times and then fail.
Every pre-publication failure closes and unlinks only the temporary name through
the retained parent descriptor. The final destination is never unlinked during
cleanup.

If the atomic rename succeeds but the parent-directory `fsync` fails, Blossom
returns a distinct `published_durability_uncertain` outcome. It must truthfully
report that the file may exist and must not retry automatically. This is not
reported as verified success.

### Overwrite, backup, and rollback

Overwrite is denied by `RENAME_NOREPLACE`, including if another actor creates
the destination after approval. Because no existing file is replaced, backup is
not applicable to this capability.

Pre-publication rollback is deletion of the private temporary file only.
Post-publication automatic rollback is prohibited: safely deleting the final
name after another actor can rename or replace it would require additional
identity-conditional kernel operations and could remove someone else's file.
The distinct durability-uncertain result preserves truthful recovery context.

A later overwrite capability requires a new accepted ADR covering conflict
tokens, backup placement and retention, atomic exchange/replacement, ownership,
permissions, rollback, and recovery UI.

### Approval, output, verification, and audit

Policy remains deny by default. The CLI registers one `ask` rule and offers only
`Approve once` or `Deny`. The exact preview displays the workspace root,
destination, directory identities, fixed permissions, byte count, digest, and
JSON-escaped complete content. Approval tokens are never printed.

The result identifies the fixed scope, created identity, byte count, digest,
mode, and durability state. Verification consumes that result without reopening
the user-supplied path.

Audit omits clear workspace path, destination, and content. It records digests
of root and relative destination, directory identities, content byte count and
digest, temporary cleanup outcome, publication state, durability state, and
verification. Partial publication is never collapsed into a generic failure.

## Alternatives considered

- Direct `openat(..., O_CREAT | O_EXCL)` at the final name: no overwrite, but
  exposes a partially written file before verification.
- Create and rename without `RENAME_NOREPLACE`: rejected because a race could
  replace an existing destination.
- Canonicalize or concatenate root and destination strings: rejected because
  lexical containment cannot enforce symlink, mount, or replacement policy.
- Bubblewrap: rejected for the native provider because safely selecting mounts
  still needs the same descriptor-relative boundary and adds process execution.
- Support overwrite immediately: rejected because backup and recovery semantics
  are materially different authority, not an option on create.
- Delete the published file when directory sync fails: rejected because a
  post-publication name can race and deletion could target a replacement.

## Security and privacy consequences

Retained descriptors and kernel resolution flags prevent root/parent path
replacement from redirecting the operation. `RESOLVE_NO_XDEV` prevents nested
mount traversal. Atomic no-replace publication ensures an existing or raced-in
destination is not overwritten. Fixed private permissions prevent accidental
group/world exposure by this tool.

The content, paths, filenames, and their digests remain private local data.
Approval must display clear scope and escaped content to the local user, while
audit remains redacted and locally access-controlled.

This capability grants no directory listing, arbitrary mode, symlink creation,
special-file access, overwrite, append, delete, rename of user files, executable
launch, network access, or privilege.

## Operational consequences

The production provider requires Linux `openat2` and `renameat2` support plus a
filesystem that supports file and parent-directory `fsync`. Unsupported behavior
fails closed; there is no legacy rename or ordinary-open fallback.

macOS runs portable request, policy, approval, verification, audit, and state
tests. Ubuntu CI must prove real root/parent containment, intermediate and final
symlink rejection, traversal rejection, nested-mount policy where permitted,
destination races, no-replace publication, permissions, bounds, cleanup,
durability, and zero executor calls.

## Migration and rollback

The tool is removable by unregistering its dedicated typed capability. A future
overwrite design must be a separate request and capability; it may not silently
expand `files.write:create`.

## Validation

- Closed schemas reject caller-selected modes, temp names, flags, roots outside
  the exact selected scope, and unknown fields.
- Approval binding/replay tests mutate root identity, parent identity,
  destination, content, digest, and mode.
- Denial, cancellation, expiry, and non-TTY tests prove no file is created.
- Linux adversarial tests cover traversal, root/parent/final symlinks, special
  files, races, collisions, existing destinations, cleanup, mode, bounds,
  mutation, publication, and directory durability.
- Audit tests distinguish pre-publication failure, durable creation, and
  published-but-durability-uncertain state without clear paths or content.
