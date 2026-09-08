# ADR-0026: Phase 9 distribution and update boundary

- Status: Accepted
- Date: 2026-09-08
- Accepted: 2026-09-08 after project-owner review of the Phase 9 scope
- Owners: Project maintainers

## Context

Blossom has reviewed application, service, shell, model-runtime, context, and
durable-memory boundaries, but it does not yet have a supported installation or
update path. The preserved prototype installers contain development assumptions
that are incompatible with those boundaries. Distribution can also silently
widen authority through package scripts, mutable downloads, service enablement,
unreviewed hardware access, or an updater that cannot recover from interruption.

Phase 9 must therefore prove a small complete distribution lifecycle before it
claims broad hardware support or a public release.

## Decision

### First supported evidence target

The first target is an x86-64 UEFI virtual machine using an Arch Linux
userspace. The repository owns the image profile, package definitions,
installation state machine, branding, first-run contract, update metadata, and
rollback logic. Every external package comes from a dated, signature-verified
Arch snapshot and is recorded in a closed manifest.

This target is evidence, not a general-purpose installer or a production
release. Physical-device support, other architectures, BIOS boot, Secure Boot,
full-disk encryption, dual boot, destructive repartitioning, and unattended
installation on an existing machine remain unsupported.

### Package and image boundary

Blossom-owned binaries, QML, service units, policy files, and model-profile
metadata are installed only through reviewed Arch packages. Package manifests
bind names, versions, architectures, installed paths, modes, owners, and
SHA-256 digests. Package installation may create fixed system identities and
directories but may not download code, models, or configuration.

The image profile contains no legacy installer, development password,
autologin, SSH enablement, remote telemetry, or mutable network installer. Build
inputs and the resulting ISO checksum are retained as evidence.

### Hardware and model selection

Hardware detection produces a closed, local, content-minimized record containing
only architecture, firmware mode, graphics class, memory class, and whether
required virtualization evidence devices are present. It exposes no serial
number, MAC address, network name, storage content, or user identifier.

Model selection is made from the existing closed provider registry. The first
image defaults to no active model. A user may select one packaged compatible
profile during first-run; selection never downloads a model or widens the
provider sandbox.

### First-run and recovery

First-run is an explicit state machine. It records completion only after locale,
user, model choice, and privacy defaults validate. Secrets are never written to
the installation manifest or logs. An interrupted first-run resumes from the
last verified non-secret checkpoint.

Recovery is available from the installation media and operates only on an
installation carrying the exact Blossom installation marker. It can inspect
state, select a previously verified system slot, and repair the boot selection.
It cannot discover and modify arbitrary host installations.

### Signed updates and rollback

Updates are offline bundles with canonical metadata signed by a documented
Ed25519 release key. The verifier binds product, channel, architecture, version,
sequence, payload digest, payload size, target slot, minimum schema, and expiry.
Unknown fields, key changes, downgrade sequences, expired metadata, digest or
size mismatch, and signature failure are rejected before writes.

The first updater uses two immutable Blossom payload slots plus one atomic active
slot marker. It writes and verifies the inactive slot, records a pending boot,
and changes the boot selection atomically. A health confirmation commits the new
slot. A missing confirmation restores the previously verified slot on the next
boot. User data and durable memory are outside the slot and are never deleted by
system rollback.

There is no background update, automatic download, hidden reboot, delta patch,
key rotation, or rollback across a data-schema migration in this slice.

## Alternatives considered

### Continue the prototype image scripts

Rejected. Their package, desktop, authentication, and update assumptions do not
implement the reviewed Blossom boundaries.

### Use a mutable online installer

Rejected. It prevents exact artifact review and makes installation depend on
current mirrors, DNS, and upstream state.

### Modify the active system in place

Rejected for the first slice. Partial replacement makes interruption recovery
and truthful rollback substantially harder than inactive-slot staging.

### Claim physical Mac support from the evidence runner

Rejected. The runner proves x86-64 Linux jobs and GPU access; it does not prove
the Blossom installer, firmware, input, power, wireless, or graphics stack on
that physical model.

## Security and privacy consequences

The image builder and updater become security-critical code. Canonical metadata,
closed paths, signature verification, monotonic sequence checks, atomic markers,
and post-write hashing reduce substitution and partial-update risk. Release
private keys and recovery material never enter the repository or CI artifacts.

The evidence key is test-only and cannot authorize a public release. A public
release requires an offline maintainer key ceremony, published fingerprint,
revocation and rotation policy, reproducible artifact review, and Phase 10
hardening.

## Migration and rollback

There is no supported predecessor installation. Phase 9 installs only onto a
blank evidence disk. Migration of prototype systems is unsupported. Within the
first installation, payload rollback changes only the active system slot. Any
future state-schema migration needs separate forward and reverse evidence before
an update may declare itself rollback-capable.

Removing Phase 9 before release removes the image and package recipes; it does
not reinterpret prototype installations as supported systems.

## Validation

Phase 9 is complete only when evidence proves:

- the package and image input manifests are closed, canonical, and digest-bound;
- image builds contain no legacy defaults, mutable downloads, enabled SSH,
  development password, or hidden network dependency;
- hardware and model selection accept only the fixed minimal schemas;
- first-run interruption and resumption preserve truthful state;
- update signatures, expiry, architecture, sequence, size, digest, slot, and
  schema are verified before staging;
- interruption at every update transition retains one bootable verified slot;
- successful health confirmation commits the update and failed confirmation
  restores the prior slot without touching user data;
- recovery rejects unmarked disks and repairs only the exact marked install;
- a fresh ISO installs in a disposable x86-64 UEFI VM, boots from its virtual
  disk, applies a signed evidence update, demonstrates failed-boot rollback, and
  then demonstrates a confirmed update; and
- protected repository checks and an independent exit audit pass.
