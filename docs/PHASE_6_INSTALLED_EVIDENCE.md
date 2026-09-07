# Phase 6 installed shell evidence

Status: complete for the Phase 6 boundary. ARM64 development evidence and the
authoritative x86-64 installed interaction gate are recorded below.

An additional Apple Silicon ARM64 VM experiment is tracked in
`docs/PHASE_6_APPLE_SILICON_VALIDATION.md`. The ARM guest reaches a serial login
prompt; owner screenshots show Hyprland rendering, DRM/VirGL diagnostics, the
installed Blossom preview, denial, Escape cancellation, and approve-once
execution with successful verification. This does not replace the separate
x86-64 evidence below.
The first ARM desktop assembly stopped at an incompatible Hyprland/Aquamarine
package transaction. A frozen snapshot subsequently passed full 279-package
authentication and image assembly; no dependency or signature check was
bypassed. This is not yet a Blossom graphical workflow pass.

The ARM experiment subsequently built the service and plugin, passed six
session-service tests and 20 core tests selected by `shell_`, and exposed an
installed-library packaging defect. The local fix colocates both plugin
libraries with `$ORIGIN` RUNPATH; a fresh ARM installed-ELF check passes.
A separate shell-test image was assembled and booted. This does not close the
x86-64 installed gate or the remaining graphical approval-flow requirements.

Subsequent owner screenshots show the ARM shell-test interface rendering, but
the service exits with status 69. The user bus responds and advertises the
service as activatable. Inspection identified `ProtectHome=yes` hiding the
required `/run/user/1000/bus` path as a likely cause. The local unit now uses
empty read-only home/runtime trees (`ProtectHome=tmpfs`) and a single required
`BindReadOnlyPaths=%t/bus` exception. Static checks require that exact exposure;
runtime activation and unrelated-file containment remain to be verified. The
running image still has the older unit until a controlled test override is
applied. No successful graphical workflow is claimed.

The owner subsequently shut down the test guest; its log reached power-down
after filesystem unmounts. A separate offline image now incorporates the
socket-only unit correction and a larger diagnostic terminal font. Installed
ELF checks, unchanged runtime package-list comparison and filesystem checks
pass. Boot, service activation and namespace containment still require runtime
evidence; prior disks remain preserved.

The socket-fixed guest subsequently reported the service started, exposed the
four methods through introspection and returned `[]` from ReadActivity1 via
busctl. The Qt client still failed. Reproduction with installed Qt 6.11.2
confirmed implicit QVariant conversion sends `i` for quint16 instead of the
required `q`. Explicit fromValue conversion now preserves version/limit types.
The local client also clears stale unavailable state after a successful activity
refresh. An offline ARM test using the real Qt client and Rust service passed
activity, preview, deny and cancel; it sent no approval. This is session-bus
protocol evidence, not installed systemd isolation or graphical execution proof.

The installed graphical preview then rendered all fixed security fields. A
denial after the screenshot round-trip closed the panel but showed unavailable.
The strengthened real-client test passed the complete post-denial refresh, so
the likely difference is the 30-second approval expiry. Expired decisions had
been collapsed into a generic D-Bus error. They now return an explicit
fail-closed `expired` outcome after recording rejection and cancellation; no
execution starts. Local and ARM core tests pass, and the real ARM Qt/Rust test
still passes activity, preview, denial, cancellation and their refreshes without
sending approval. The rebuilt guest then passed installed graphical denial:
the panel closed, status became `denied`, and correlated sequences 1–4 showed
request accepted, policy ask, approval issued and terminal denial. No execution
or verification event appeared. Graphical expiry remained to be tested at that
point.

The first installed Escape test exposed a real focus defect: the approval
overlay was visible, but its window-level `Keys` handler had no active-focus
item. The corrected panel requests window activation whenever it becomes
visible, explicitly focuses a `FocusScope`, and also supplies a non-repeating
window Escape shortcut. Static checks pin those requirements. A fresh offline
ARM64 image retained the exact package list and passed ELF, filesystem, and
exported-artifact checksum checks. In the booted corrected image, Escape closed
the approval panel, status became `cancelled`, and correlated sequences 1–4
showed request acceptance, policy ask, approval issuance, and terminal
cancellation. No execution or verification event appeared.

The first approve-once attempts exposed three independent sandbox/service
integration conflicts. Bubblewrap requires `AF_NETLINK` while constructing its
private network namespace and uses `openat2()` for safe path resolution, so the
unit now permits only `AF_UNIX AF_NETLINK` and does not apply
`RestrictSUIDSGID=`. After those corrections, an exact production-profile probe
showed `--disable-userns` trying to write the deliberately protected
`/proc/sys/user/max_user_namespaces`. That option is redundant for this closed
executor: the executable and arguments are fixed to `/usr/bin/uname -s`, the
user and other namespaces remain private, all capabilities are dropped, `/usr`
is read-only, and no network, procfs, device tree, or writable temporary
filesystem is exposed. The corrected profile therefore omits only that
conflicting write.

A fresh offline ARM64 image passed all six feature-enabled service tests, ext4
filesystem checking, and exported SHA-256 verification. Its root filesystem
hash is `36d540929b073e387c095406e0710e529ac736364acb09a4b75184583f1cd59b`.
In the booted image, the owner selected approve once and the UI reached
`verified`; correlated sequences 1–7 show request acceptance, policy ask,
approval issuance, one-time approval, execution start, execution finish, and
terminal verification for one request ID. This is installed ARM64 graphical
approve-once evidence.

The first no-touch graphical expiry test exposed a presentation-lifecycle bug:
the backend rejected a late decision, but the native client never scheduled a
deadline transition, so an untouched panel stayed visible. The client now arms
a bounded one-shot timer from the fixed preview deadline. Its timeout sends the
same bound cancellation request; the service remains the authority that checks
the deadline, records expiry, consumes the pending approval, and starts nothing.
Malformed or unreasonably distant deadlines fail closed in the client.

The rebuilt offline ARM64 image passed native compilation with warnings treated
as errors, ext4 checking, and every exported SHA-256 check. Its root filesystem
hash is `c83bdccffe5163ebe47234bf3044f969f282e96a679cbef4f94ce658cba9302b`.
In the installed graphical run, the untouched panel closed after its deadline,
status became `expired`, and each observed request ended with expiry and terminal
cancellation after request, policy, and approval issuance. No approval grant,
execution, or verification record appeared.

The first graphical service-loss run exposed another stale-client-state defect:
the D-Bus broker was stopped while approval was pending, but the panel remained
visible because the native client did not watch ownership of the fixed bus name.
The client now uses `QDBusServiceWatcher` for unregistration of only
`org.blossomos.Shell1`; owner loss stops the local expiry timer, clears the
preview, closes the panel, and reports `unavailable`. It does not restart the
service or transfer any authority into QML.

A deterministic test-only user unit in the ignored disposable image stopped the
broker repeatedly; neither that unit nor its script is part of the tracked
product. The rebuilt ARM64 plugin compiled with warnings treated as errors, and
the image passed ext4 and exported SHA-256 checks. Its root filesystem hash is
`23883d2dd8979d0ee6c95931b5a07fdbd199f47d8f573ea89630d49f655613cb`.
In the installed graphical run, the open approval panel disappeared within the
bounded test interval and status became `unavailable`. The activity projection
was empty because its authoritative in-memory service had been deliberately
terminated; this evidence therefore proves fail-closed client presentation, not
durable audit persistence across broker loss. ADR-0021 deliberately excludes
durable activity recovery; service replacement and bus-loss tests instead prove
that old previews fail closed and no execution starts.

## Passing x86-64 installed compatibility evidence

The final authoritative run is GitHub Actions run
[`34102303699`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34102303699),
which passed on 2026-09-07 at signed commit
`b3bd89eb30859bfc89aab98202e010b7825b9a17`. Its installed Arch x86-64 job
completed in 4 minutes 6 seconds on the trusted `blossom-gpu` runner.

The run passed the real Qt/Rust wire path, fixed Bubblewrap networkless
`/usr/bin/uname -s` executor, package installation, service activation, and
installed Hyprland/Quickshell surface. It then passed standard Qt accessibility
and Blossom AT-SPI checks, compositor-close and Escape cancellation,
keyboard-only denial and approve-once verification, and post-decision refresh.
The accessibility probe observed the 14 fixed security fields, safe initial
focus, keyboard activation, and denied and verified outcome announcements.

The interaction groups restart only the installed UI client while preserving
the authoritative service and its correlated audit chain. This isolates window
lifecycle state without replacing the policy, token, executor, verification, or
audit authority. Hidden approval state is asserted through AT-SPI
`STATE_SHOWING`; it is not inferred from a destroyed accessibility object.

This run supersedes the earlier compatibility-only run below for the Phase 6
exit decision. Its evidence remains deliberately narrow: it does not prove
broad hardware, physical input/display, installer, distribution image, or
release readiness.

GitHub Actions run `34032625185` passed on 2026-09-06 at signed commit
`41a84f7c4700242ee5d7f11fdce2c91c87b0155d`. The job ran on the trusted,
ephemeral `blossom-x64-phase6` Linux runner and used the digest-pinned official
Arch container declared by the workflow. Every workflow step completed in
4 minutes 18 seconds.

The run proved the exact production package versions, built the feature-gated
Rust service and native QML plugin, passed the real Qt client/Rust service wire
test, installed the closed package tree, and gave the non-root test account
read/write access to the runner's DRM render node. It verified the host Wayland
protocol floor, launched the separately pinned evidence-only Niri parent on its
dynamically discovered socket, verified the nested compositor, shell, seat, and
DMA-BUF protocol floors, and then launched the real pinned Hyprland and
Quickshell. Quickshell loaded the installed Blossom QML/plugin and successfully
activated `org.blossomos.Shell1`; the job rejected missing QML modules,
unavailable types, and root-component creation failure.

This earlier run is retained as compatibility history. It does not add Niri to
the production package set. Run `34102303699` is the authoritative installed
interaction evidence; the hardware, installer, distribution, and release
limitations below still apply.

`.github/workflows/phase6-installed-evidence.yml` is a manually dispatched,
owner-provided GPU-runner gate. It creates a disposable official Arch x86-64
userspace and fails unless the installed Hyprland, Quickshell, systemd, and
dbus-broker versions exactly match the accepted production lock. The workflow
also pins its evidence-only Niri compatibility parent separately; this does not
widen the closed production package set. The workflow builds the feature-gated Rust
session service and native QML plugin, installs the fixed binary, user unit,
D-Bus activation metadata, plugin module, and QML files under their intended
root-owned paths, and checks unit and ELF dependencies.

The runner must expose `/dev/dri` and a logged-in Wayland desktop socket at
`/run/user/1000/wayland-0` to the job container. Only that socket is mounted;
the rest of the host runtime directory is not exposed. The harness verifies
that the parent advertises `wl_compositor` and `xdg_wm_base` version 6 or newer,
then starts the pinned evidence-only Niri parent nested in that desktop. The
harness requires the nested parent to expose `wl_compositor` and `xdg_wm_base`
version 6 or newer, `wl_seat` version 9 or newer, and Linux DMA-BUF before it
starts the real pinned Hyprland. Hyprland launches the real pinned Quickshell,
which loads the installed Blossom QML and native plugin. The QML performs a
bounded activity refresh, requiring the fixed D-Bus service to activate
successfully. The harness rejects missing parent protocols, missing QML modules,
unavailable types, or root-component creation failure.

Hosted-runner investigation is recorded by failed run `33890423701`: Cage and
Hyprland were real installed binaries, but the hosted container had no DRM
render node. Cage therefore used its Pixman/SHM allocator, while Aquamarine
requires `zwp_linux_dmabuf_v1` and refused to start. That run is diagnostic
evidence only and is not a passing installed-runtime claim.

Reproduction requires a trusted GitHub Actions runner with labels `self-hosted`,
`linux`, `x64`, and `blossom-gpu`, from a dedicated installed test machine's
logged-in Wayland session, with Docker permission to pass `/dev/dri` and its
display socket into the container. Do not use a general-purpose personal
workstation or a runner holding unrelated secrets.

The passing run establishes Arch userspace ABI, packaging, activation, nested
compositor, and configuration-load evidence. It exercises a DRM render node but
does not prove broad GPU compatibility, physical displays or input devices,
compositor security, an Arch
kernel, installer, ArchISO, upgrade/rollback, distribution packaging, or release
readiness. Escape and compositor-close cancellation, keyboard-only denial and
approve-once execution, accessibility, no-touch expiry, and fail-closed
service-loss presentation are exercised. Activity recovery is intentionally
absent under ADR-0021; broad hardware, installer, distribution-image, and
release proof remain outside this Phase 6 gate.
