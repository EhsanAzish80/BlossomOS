# ADR-0005: Linux `openat2` and a pinned descriptor for exact-file reads

- Status: Accepted
- Date: 2026-09-02
- Owners: Project maintainers

## Context

ADR-0004 permits one user-selected single-file read only after exact-path
containment is proven. A lexical path check followed by `File::open` is not
sufficient: a symlinked component can redirect resolution, and the selected
path can be replaced between approval and opening. Holding a pathname alone
does not bind approval to the object the user selected.

This first file capability should establish a narrow reusable read boundary
without introducing a generic filesystem API, directory traversal, binary
terminal output, write access, or caller-configurable sandbox rules.

## Decision

Implement one Linux-only `files.read:content` tool for bounded UTF-8 text. Its
scope is one absolute user-selected path and one selected file identity.

### Selection and request binding

The interactive CLI accepts one absolute path. It rejects empty and relative
paths, NUL or control characters, `.` and `..` components, trailing separators,
and any input beyond the path bound. No path normalization may silently change
what the user selected.

Before policy approval, the selection layer opens `/` as a directory and calls
Linux `openat2` on the relative remainder with code-owned flags:

- `O_RDONLY | O_CLOEXEC | O_NOCTTY | O_NONBLOCK`; and
- `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS`.

Failure or lack of `openat2` support fails closed. Blossom does not fall back to
ordinary path resolution. `RESOLVE_NO_XDEV` is not used because an exact file on
a mounted user filesystem remains a valid selection; authority is still limited
to the single opened object.

The selection must be a regular file. The code records device, inode, size,
modification time, and change time from the opened descriptor. These values and
the exact path form the typed request scope, so the approval token is bound to
both the displayed path and selected identity. The opened descriptor is retained
privately by the provider; it is never serialized, printed, or supplied by a
caller.

Opening and statting the selected object occurs before the approval prompt so
Blossom can display and bind the actual resource. No content byte is read before
approval. Denial, cancellation, expiry, or non-interactive invocation closes the
descriptor and produces no content result.

### Approved read

After once-only approval, the provider checks the retained descriptor against
the bound identity, reads from that same descriptor exactly once with a
64-KiB-plus-one enforcement read, then checks identity again. Any identity
change, oversize result, read failure, invalid UTF-8, or non-regular target fails
closed. The result includes the exact path, UTF-8 content, byte length, SHA-256
digest of the exact bytes, and identity provenance.

The CLI renders content through escaping that cannot emit untrusted terminal
control sequences. It does not interpret markup, shell syntax, ANSI escapes, or
file contents as instructions.

### Policy, verification, and audit

The policy engine remains deny by default. The CLI registers only an `ask` rule
for `files.read:content`; there is no permanent approval. The exact preview
shows path, identity, maximum bytes, native mechanism, and excluded authority.

Verification uses the returned typed scope and digest without reopening the
path. Audit records approval transitions, a digest of the selected path, device
and inode, byte count, content digest, and verification result. File content and
the clear path are omitted from audit activity.

## Alternatives considered

- Canonicalize then open: rejected because it follows symlinks and leaves a
  selection-to-open race.
- Walk components with repeated `openat`: possible, but `openat2` expresses the
  complete no-symlink resolution policy atomically in the Linux kernel.
- Bubblewrap plus a read-only bind: rejected for this native read because mount
  setup adds process execution and still requires safely selecting the bind
  source first.
- Open only after approval: rejected because approval would bind only a mutable
  pathname, not the selected object.
- Read content before approval and bind its digest: rejected because denial
  would occur after the sensitive content had already been read.
- Support arbitrary binary files: deferred. This checkpoint is bounded UTF-8
  text so output can be represented and reviewed without a new binary transport.

## Security and privacy consequences

Kernel-enforced no-symlink resolution and the retained descriptor prevent path
replacement from redirecting the approved read. Pre-read and post-read identity
checks detect in-place metadata changes during the approval/read window. They do
not make concurrently mutable files transactional; a concurrent write is a
typed failure, not a partial success.

The selected path, file identity, content, and digest are private local data.
The human preview necessarily displays the clear selected path. Audit redaction
does not make low-entropy path or content digests anonymous, so audit storage
must remain local and access-controlled under the broader audit design.

This decision grants no directory listing, globbing, relative lookup, symlink
following, special-file access, write access, command execution, network access,
or privilege.

## Operational consequences

The production provider is Linux-only and requires a kernel with `openat2`.
Portable parser, policy, approval, verification, and audit tests continue to run
on macOS. Ubuntu CI must exercise the production provider and adversarial
fixtures for intermediate/final symlinks, traversal, replacement, FIFOs,
sockets, directories, oversize files, mutation, malformed UTF-8, and unavailable
paths.

## Migration and rollback

The provider is behind a dedicated file-selection/read trait. A future
confinement mechanism may supersede this ADR without widening the request. The
tool can be removed by unregistering its typed capability; no generic fallback
is permitted.

## Validation

- The request parser rejects unknown fields and every non-exact path form.
- Approval binding and replay tests cover the path and full selected identity.
- Linux integration tests prove no-symlink resolution and retained-descriptor
  behavior under path replacement.
- Denial, cancellation, expiry, and non-TTY tests prove no content bytes are
  read and no executor is called.
- Result verification and audit tests prove bounds, provenance, digest, and
  content/path redaction.
