# Phase 2 capability and sandbox baseline

Status: completion baseline prepared on 2026-09-02. Phase 2 is complete when
this exit audit passes the protected pull-request checks and merges to `main`.

## Scope

Phase 2 extends the deterministic Phase 1 security path with nine closed,
typed capabilities. It does not add an LLM, Bash interface, generic executable,
sudo path, privileged helper, IPC daemon, Hyprland integration, or Quickshell.
The preserved Python/XFCE prototype remains historical pre-alpha material and
is not part of this trusted Rust boundary.

All request JSON is capped at 512 KiB before parsing. Request identifiers, tool
names, resource selectors, inputs, and results have narrower tool-owned bounds.
Unknown fields are rejected. The broker derives capabilities exhaustively from
the closed `ToolRequest` enum; callers cannot assert capabilities or configure
execution and confinement.

## Registered capability evidence

| Capability | Default policy | Code-owned operation and confinement | Verification and audit | Target-Linux evidence |
| --- | --- | --- | --- | --- |
| `system.read:kernel.identity` | Ask once | Fixed `/usr/bin/uname -s` only; fixed Bubblewrap profile, read-only system view, no network, cleared environment, dropped capabilities, bounded time/output | Exact command result verifier; hash-chained content-redacted transitions | Phase 1 protected Quality workflow and real Bubblewrap integration test |
| `system.read:os.identity` | Allow | One bounded open/read of `/etc/os-release`, falling back to `/usr/lib/os-release` only when absent; regular final target; no shell evaluation | Allowlisted schema and exact-byte provenance verified without reopening; identity values omitted from audit | PR #9 / `048393b`; `docs/PHASE_2_OS_IDENTITY.md` |
| `system.read:uptime` | Allow | One bounded retained-descriptor read of `/proc/uptime`; native parser, no subprocess | Duration/source/digest invariants verified without reopening; duration omitted from audit | PR #10 / `84f64ec`; `docs/PHASE_2_UPTIME.md` |
| `system.read:memory.summary` | Allow | One bounded retained-descriptor read of `/proc/meminfo`; fixed allowlist and checked arithmetic | Schema/relationships/source/digest verified; memory values omitted from audit | PR #11 / `8c9ed6d`; `docs/PHASE_2_MEMORY_SUMMARY.md` |
| `system.read:storage.summary` | Allow | Native `statvfs` of fixed `/` scope; checked conversions; no command | Capacity relationships and fixed provenance verified; capacity values omitted from audit | PR #12 / `b5418b0`; `docs/PHASE_2_STORAGE_SUMMARY.md` |
| `process.read:self` | Allow | Native calling-process APIs only; no procfs enumeration or subprocess | Minimal PID/effective-user schema verified; identifiers omitted from audit | PR #13 / `8c791f8`; `docs/PHASE_2_PROCESS_SELF.md` |
| `process.read:list` | Ask once | Same-effective-user, bounded `/proc/<pid>/status` records only; no command line, environment, files, sockets, or memory | Sorted unique bounded result plus typed skipped/truncated state; audit stores counts only | PR #14 / `05d83d2`; `docs/PHASE_2_PROCESS_LIST.md` |
| `files.read:content` | Ask once | Exact absolute regular file selected before approval; Linux `openat2` rejects symlink components; retained identity; 64-KiB UTF-8 maximum | Content and provenance verified from retained read; clear path/content omitted from audit | ADR-0005, PR #16 / `58a7700`; `docs/PHASE_2_EXACT_FILE_READ.md` |
| `files.write:create` | Ask once | Exact workspace root and relative destination; retained directories; no symlinks/mount crossing; unnamed `O_TMPFILE`; fixed `0600`; atomic no-replace publication | Retained inode/content/mode/digest and durability verified; paths/content omitted from audit | ADR-0007, PR #19 / `5491d78`; `docs/PHASE_2_WORKSPACE_FILE_CREATE.md` |
| `services.read:status` | Ask once | Exact `.service` on fixed system bus; `GetUnit` plus four fixed properties; `NoAutoStart`; one three-second operation deadline; no generic D-Bus | Fixed provenance and bounded state schema verified without a second observation; unit/state values omitted from audit | ADR-0008, PR #21 / `c384cb0`; `docs/PHASE_2_SERVICE_STATUS.md` |

ADR-0004 named the eighth checkpoint `files.write:content`. ADR-0006 narrowed
that authority before implementation to create-only `files.write:create`, and
ADR-0007 superseded its publication mechanism before merge. No broad content
write, overwrite, append, rename, or deletion capability exists.

## Adversarial and failure coverage

The capability evidence documents record malformed, unavailable, bounded-output,
and platform-specific cases for each resource class. The protected Linux suite
also exercises, where applicable:

- denial, cancellation, expiry, replay rejection, and non-interactive refusal;
- executor/provider call counts proving denied work does not start;
- malformed/oversized input and output, invalid UTF-8, NUL and control data;
- `/proc` disappearance, PID substitution, same-user filtering, special files,
  symlinks, and bounded enumeration;
- exact-file traversal, intermediate/final symlinks, path replacement, FIFO and
  identity-change behavior;
- workspace root/parent/final symlinks, mount containment, retained-directory
  replacement, destination races, no-overwrite publication, private mode,
  unnamed temporary inode behavior, and explicit uncertain durability; and
- fixed D-Bus destination/path/interfaces/methods/properties, unavailable units,
  malformed states, disconnect/failure behavior, and total-operation timeout.

## Cross-tool composition review

The combined Phase 2 set can reveal bounded host identity, resource summaries,
same-user process names, one approved file, and one approved service state. This
is private metadata, so process listing, file access, workspace creation, and
service status require exact once-only approval. Lower-sensitivity singleton
system summaries are explicit policy allows but still pass through policy,
verification, and audit.

Composition does not create generic execution or ambient filesystem authority:

- no native result is interpreted as an executable, argument vector, shell
  source, environment, mount, namespace, D-Bus destination, or privilege request;
- the only command-backed request remains the fixed Phase 1 diagnostic;
- file read cannot list directories or choose paths after approval;
- workspace write can only create one absent private UTF-8 file and cannot
  overwrite, append, delete, chmod, traverse, or execute it; and
- service status cannot enumerate, load, start, stop, restart, or mutate units.

Any future bridge from observed data to a new action requires a separately typed
capability, policy decision, verifier, audit treatment, adversarial tests, and
an ADR when it changes a trust boundary.

## Repository and quality evidence

- ADR-0004 is accepted; ADRs 0005, 0007, and 0008 own the added confinement
  boundaries. ADR-0006 is explicitly superseded by ADR-0007.
- Each capability was introduced in expansion order through an independently
  reviewed pull request and protected Linux checks.
- The Quality workflow runs repository policy checks, prototype smoke tests,
  ShellCheck, Rust formatting, strict Clippy, all Rust targets, and real Linux
  boundary tests with Bubblewrap and a controlled D-Bus daemon.
- Separate protected workflows run Gitleaks and CodeQL. Dependabot tracks the
  locked Cargo and GitHub Actions dependency graphs.
- The completion change adds a registry-wide test proving every request variant
  maps to its exact static capability and remains denied by an empty policy.

## Exit decision

After this baseline passes protected review and merges, every registered tool
has a statically derived capability, code-owned operation boundary, deterministic
verification, redacted audit behavior, and applicable containment tests. The
Phase 2 exit gate is satisfied. Phase 3 remains separate and must begin with its
own privileged-helper design and threat review.
