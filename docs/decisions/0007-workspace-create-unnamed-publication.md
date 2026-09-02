# ADR-0007: Publish workspace files from an unnamed retained inode

- Status: Accepted
- Date: 2026-09-02
- Owners: Project maintainers
- Supersedes: ADR-0006 publication and temporary-cleanup mechanism

## Context

ADR-0006 selected a verified named temporary file followed by
`renameat2(RENAME_NOREPLACE)`. Implementation review found a remaining race in
an adversarial writable parent directory: after Blossom verifies its retained
temporary descriptor, another writer can rename that temporary pathname away
and place a different inode at the same name before `renameat2`. The rename
would then publish an inode Blossom did not verify.

Reopening and comparing the temporary name cannot close the race because the
name may change again between comparison and rename. Publication must operate
on the exact retained inode, not on a mutable source pathname.

## Decision

Retain ADR-0006's typed request, root/parent selection, content and mode bounds,
approval, result, audit, no-overwrite rule, and durability semantics. Replace
its named temporary file and rename publication with:

1. create an unnamed regular inode in the retained parent filesystem using
   descriptor-relative `openat` with `O_TMPFILE | O_RDWR | O_CLOEXEC` and mode
   `0600`;
2. enforce exact `0600`, write, `fsync`, read back, and verify content, digest,
   type, identity, length, and permissions through that retained descriptor;
3. publish that exact descriptor with
   `linkat(temp_fd, "", parent_fd, final_name, AT_EMPTY_PATH)`; and
4. `fsync` the retained parent directory before returning durable success.

`linkat` to a new name is atomic and fails with `EEXIST` rather than replacing
an existing or raced-in destination. `AT_EMPTY_PATH` binds the operation to the
retained descriptor, so there is no source pathname to substitute.

The production provider fails closed when `O_TMPFILE` or `AT_EMPTY_PATH`
publication is unsupported by the kernel or filesystem. There is no named-temp,
`renameat2`, ordinary-create, `/proc/self/fd`, or unsandboxed fallback.

## Cleanup, rollback, and partial publication

Before publication, closing the unnamed descriptor automatically discards the
unlinked inode. No cleanup pathname exists, so Blossom cannot delete an
attacker-controlled collision and no temp files can be left in the workspace.

After successful `linkat`, the final name exists. As in ADR-0006, parent
directory sync failure returns `published_durability_uncertain`; Blossom does
not delete or retry the published name. Overwrite and backup remain out of
scope because this capability only creates an absent destination.

## Alternatives considered

- Reopen and compare the named temp immediately before rename: rejected because
  comparison and rename remain separate operations on a mutable name.
- Lock the parent directory: Linux advisory locks do not prevent unrelated
  pathname operations and mandatory locking is unsuitable.
- Publish through `/proc/self/fd/<fd>`: rejected because it relies on procfs
  magic-link behavior expressly excluded from the path boundary.
- Add an unsafe local syscall wrapper: unnecessary because the pinned safe `nix`
  API exposes `O_TMPFILE`, `linkat`, and `AT_EMPTY_PATH`.

## Security and privacy consequences

The verified inode is now the inode published. A concurrent writer can cause a
no-replace conflict but cannot substitute content or make Blossom overwrite the
destination. Removing named temporary files also removes predictable-name
collision and cleanup-race surfaces.

Filesystem support is narrower. This is an intentional fail-closed operational
constraint, not permission to fall back to a weaker publication path.

## Migration and rollback

No released format or API migration is required. ADR-0006 had no merged
implementation. Its publication mechanism is superseded before the capability
is enabled.

## Validation

- Linux tests race destination creation and prove `EEXIST` leaves it unchanged.
- An adversarial test confirms no `.blossom-tmp-*` names are created.
- Descriptor verification proves the linked inode and digest match the retained
  unnamed file.
- Unsupported `O_TMPFILE` or `AT_EMPTY_PATH` behavior fails before publication.
- Directory-sync failure remains a distinct published-but-uncertain result.
