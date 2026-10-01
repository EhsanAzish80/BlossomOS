# ADR-0036: Bounded workspace plan approval

Status: Proposed

## Context

Blossom currently approves one prepared effect at a time. That is appropriate
for creating a single workspace file, but it does not scale to useful file
organization. Asking for a password for every move would train people to
approve prompts without reading them, while allowing a model or shell client to
submit a mutable batch would destroy the exact-effect guarantee established by
ADRs 0031, 0032 and 0034.

The intended investigative-workflow direction separates observation,
interpretation and authority:

1. trusted local code observes bounded workspace facts;
2. optional local models may propose labels or a grouping scheme;
3. trusted code constructs an exact list of filesystem effects;
4. the broker displays and retains that exact prepared plan;
5. one password-backed decision approves only that plan; and
6. trusted code executes, verifies and records the result.

Observed metadata, model labels and model-generated folder names are not
authority. File content may contain adversarial instructions. A plan must remain
safe even when every label or grouping suggestion is wrong.

ADR-0020's general orchestration plan remains limited to 16 heterogeneous
steps, with policy and approval evaluated per step and no automatic rollback.
This ADR defines a different, narrower transaction type: up to 128 homogeneous
workspace organization effects under one fixed capability, one workspace root
and one exact preview. Its single plan approval and code-owned rollback do not
authorize approval reuse or automatic rollback for general orchestration.

## Decision

### First capability

The first plan-capable release supports deterministic workspace organization by
observed facts only. It may propose regular-file moves and renames based on:

- a bounded content digest for duplicate detection;
- a locally observed file type, with the detector and observation source shown;
- bounded filename patterns; and
- available filesystem or embedded metadata dates, labelled by their source.

These observations are evidence, not guaranteed truth. Embedded dates and type
detection may be absent, ambiguous or forged. Ambiguous items remain in place
and appear under `Needs review`; Blossom does not silently guess.

Duplicate detection is report-only in the first release. It never deletes,
deduplicates, replaces or moves a file merely because its digest matches
another file.

Embeddings, learned classifiers, document-content extraction and LLM-generated
group names are outside this ADR's first implementation. They may later supply
untrusted labels or schemes, but they may never construct or execute effects.

### Closed plan shape

A plan is a non-empty, canonically serialized list of at most 128 effects. The
only admitted effects in the first release are:

- create one workspace-relative directory beneath a retained existing parent;
  and
- rename or move one regular file between workspace-relative paths on the same
  mounted filesystem.

A created directory has exactly one safe path component. Nested organization
is represented by ordered directory effects, each depending on its parent.
Plans never remove a pre-existing directory or delete a regular file.

Every prepared effect contains trusted, derived fields that wire input cannot
supply:

- the retained workspace root identity and mount identity;
- the source parent and destination parent authorities;
- the normalized source and destination paths;
- the source device, inode, size, modification time, change time and content
  digest observed during preparation;
- the destination non-existence observation;
- the proposal origin and observation sources; and
- a stable effect identifier used only within the plan.

The plan also contains its schema version, creation and expiry times, effect
count, plan origin, and a digest over the complete canonical serialization.
List order is significant and is included in the digest.

Wire types contain only the user's request or untrusted proposals. Domain plan
types do not implement `Deserialize`, `Clone` or public constructors. One
trusted resolver is the only bridge from proposals to a prepared plan.

### Preparation and retained authority

Admission capacity is reserved before the resolver opens files or directories.
The prepared plan owns the capacity reservation and all retained authority by
value. Denial, cancellation, expiry, peer disconnect, authentication failure
and replacement remove the plan from the store; Rust ownership then closes its
descriptors and returns its capacity automatically.

The broker permits one pending plan per peer and a small global number of
pending plans. The descriptor budget is derived from the maximum effect count
and is enforced before preparation. Capacity exhaustion fails closed, does not
evict an existing plan, and is visible in the activity surface.

Preparation resolves every path beneath the retained workspace root without
following symlinks. It rejects special files, mount crossings, duplicate source
paths, duplicate destinations, source/destination overlap that would make plan
order ambiguous, undeclared parent dependencies and any path outside the
workspace. A destination must either have a retained existing directory parent
or depend on an earlier directory-creation effect. Directories themselves are
not moved in the first release.

### Exact plan preview and decision

The broker-owned approval window shows the original request and the complete
ordered effect list. It shows full workspace-relative source and destination
paths without elision, the number of unchanged `Needs review` items, conflicts,
the plan origin and expiry. A summary never substitutes for the complete list.

The window supports keyboard navigation, search within the visible plan and a
clear `Approve this plan once` action. Search and scrolling change only the
presentation; they cannot alter the retained plan or its digest.

The client returns only the opaque request identifier and exact preview digest.
The broker performs one PolicyKit `auth_self` challenge for the uniquely
verified active local graphical session and binds success to that request and
plan digest. `auth_self_keep` is forbidden. A late authentication result loses
to expiry. Denial and cancellation remain unauthenticated.

One successful password challenge authorizes one consumption of the stored
prepared plan. The shell never returns an effect list, replacement plan or
filesystem path with its decision. Replays, mutated digests, stale requests and
cross-peer decisions are rejected.

### Execution and change detection

Execution consumes the stored prepared plan by value. It never resolves a new
plan from wire input and never asks a model to repair a stale plan.

Immediately before each effect, trusted code rechecks the source through the
retained workspace and parent authority and compares every recorded identity
field and content digest. It rechecks that the destination is absent, beneath
the same workspace root and on the same filesystem. Any mismatch stops the
plan before that effect and starts rollback of earlier effects.

Each directory is created relative to retained authority with no replacement,
then opened and verified before a dependent move may use it. Each move uses the
platform's no-replace atomic rename operation relative to retained directory
descriptors. Execution never invokes a shell, follows a symlink, overwrites a
destination, copies across filesystems or deletes file content.

Linux does not provide a general rename-by-open-file-descriptor operation. A
same-user process can race a directory entry between the final identity check
and `renameat2`. The executor therefore verifies the moved object immediately
after every rename. An identity mismatch triggers rollback and an
`indeterminate` result; it is never reported as success. This residual
same-user race is part of the known trust gap and must be stated in evidence.

### Journal, rollback and verification

Before the first move, trusted code durably writes an encrypted, content-free
transaction journal on the state partition. The journal contains the plan
digest, ordered relative paths, identities, lifecycle state and integrity
chain. It never stores file contents, extracted text or model prompts.

The journal is updated and synchronized after every completed effect. On
failure, the executor walks completed effects in reverse order using no-replace
atomic renames and verifies the restored identities. A directory created by the
plan is removed during rollback only if it is still empty and has the exact
recorded identity. A rollback conflict never causes an overwrite or removal of
user content. It produces an `indeterminate` result with a visible recovery
record.

A plan is `verified` only when every destination contains the expected identity
and digest, every source path is absent, the workspace and mount identities are
unchanged, and the committed journal record is durable. Partial execution,
failed verification or incomplete rollback can never become success.

Undo is a new plan, not an unchecked journal replay. Trusted code reads the
committed journal, resolves and revalidates the inverse effects against current
state, displays the complete inverse plan and requires a new password-backed
approval. If any destination has changed, a restored source name is occupied or
a plan-created directory is no longer empty, undo stops and reports the
conflict without overwriting or removing anything.

### Audit and privacy

Audit records contain the plan digest, effect count, origin, lifecycle result,
verification result and bounded conflict codes. They do not contain filenames,
paths, file contents, extracted text, embeddings or folder names.

The approval preview necessarily displays paths to the local user, but neither
the shell activity projection nor persistent audit becomes a file inventory.

Future embedding caches are sensitive derived material. A later ADR must define
their encrypted storage, receipt-bound model identity, content-hash binding,
invalidation, deletion and background-ingestion limits before they are added.

## Alternatives considered

### Approve every move separately

Rejected for organization plans. It creates approval fatigue and makes it hard
to understand the overall transformation. Individual effect approval remains
appropriate for isolated actions.

### Approve a summary and generate moves afterward

Rejected. The user would approve a concept rather than exact effects, leaving
the model or resolver free to change the plan after authorization.

### Treat an undo journal as automatic authorization

Rejected. Workspace state may change after the original plan. Undo therefore
requires the same preparation, full preview and fresh approval as any other
effectful plan.

### Begin with embeddings or classifiers

Rejected. Deterministic organization delivers useful behavior while exercising
the plan, rollback and verification machinery without introducing derived-data
storage or model-correctness questions.

## Qualification requirements

Before the plan executor is enabled, tests must prove:

1. the preview digest changes for any effect, order, path, identity, origin or
   expiry change;
2. wire input cannot supply roots, identities, digests, modes, destinations or
   retained authority;
3. traversal, absolute paths, symlinks, special files, mount crossings,
   duplicate sources, duplicate destinations, existing destinations and plans
   over the effect limit fail before authentication;
4. one password challenge consumes exactly one stored plan, while mutation,
   replay, cross-peer decisions and late authentication produce zero effects;
5. source changes, hard-link content changes, destination creation and parent
   substitution between preparation and execution fail closed;
6. each admitted move uses no-replace atomic rename and no shell or generic
   command executor;
7. injected failure after every possible effect boundary either restores the
   original state completely or reports `indeterminate` with a recovery record;
8. verification cannot report success for partial execution, identity drift,
   missing durability or incomplete journal state;
9. undo is prepared as a fresh inverse plan, refuses changed files and
   collisions, and requires another password;
10. deny, cancel, expiry, authentication failure, replacement and disconnect
    return descriptor and plan capacity to baseline;
11. audit fixtures contain no content, prompt or path values, while the
    recovery journal contains paths only inside its authenticated encrypted
    payload and leaves no plaintext temporary file; and
12. the approval accessibility gate exposes every full source and destination
    path, the total effect count, plan origin, expiry and all conflicts without
    elision.

The installed Linux gate must additionally exercise interruption and restart
recovery from each durable journal state. A manual desktop check must confirm
that a person can inspect a maximum-size plan and that each new plan prompts for
its password again.

## Consequences

This design makes a bounded group of moves one understandable authorization
unit without granting the model batch filesystem authority. It also adds
substantial transaction, recovery and interface work before file organization
can ship.

The first useful version is deliberately deterministic and conservative.
Items may be left in `Needs review`, duplicate handling is informational, and
some otherwise valid organization requests will be refused when they cross a
filesystem or cannot be made collision-free. Those refusals are preferable to
silent guesses or irreversible effects.
