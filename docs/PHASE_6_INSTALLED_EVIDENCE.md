# Phase 6 installed shell evidence

Status: partial ARM64 installed graphical evidence recorded; the x86-64 gate
and complete graphical matrix remain open.

An additional Apple Silicon ARM64 VM experiment is tracked in
`docs/PHASE_6_APPLE_SILICON_VALIDATION.md`. The ARM guest reaches a serial login
prompt; owner screenshots show Hyprland rendering, DRM/VirGL diagnostics, the
installed Blossom preview, denial, Escape cancellation, and approve-once
execution with successful verification. This does not
replace the x86-64 gate below or require registering a personal Mac as a CI
runner.
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
durable audit persistence across broker loss. Assistive technology, broader
input compatibility, durable audit recovery, and the x86-64 gate remain open.

`.github/workflows/phase6-installed-evidence.yml` is a manually dispatched,
owner-provided GPU-runner gate. It creates a disposable official Arch x86-64
userspace and fails unless the installed Hyprland, Quickshell, systemd, and
dbus-broker versions exactly match the accepted production lock. Evidence-only
parent compositor versions are kept in a separate lock and do not widen that
closed production package set. The workflow builds the feature-gated Rust
session service and native QML plugin, installs the fixed binary, user unit,
D-Bus activation metadata, plugin module, and QML files under their intended
root-owned paths, and checks unit and ELF dependencies.

The runner must expose `/dev/dri` to the job container. Weston provides the
outer headless display, current Cage/wlroots provides the DMA-BUF-capable nested
Wayland parent required by Aquamarine, and the real pinned Hyprland runs inside
it. Hyprland launches the real pinned Quickshell, which loads the installed
Blossom QML and native plugin. The QML performs a bounded activity refresh,
requiring the fixed D-Bus service to activate successfully. The harness rejects
missing QML modules, unavailable types, or root-component creation failure.

Hosted-runner investigation is recorded by failed run `33890423701`: Cage and
Hyprland were real installed binaries, but the hosted container had no DRM
render node. Cage therefore used its Pixman/SHM allocator, while Aquamarine
requires `zwp_linux_dmabuf_v1` and refused to start. That run is diagnostic
evidence only and is not a passing installed-runtime claim.

Owner action: register a trusted, ephemeral GitHub Actions runner with labels
`self-hosted`, `linux`, `x64`, and `blossom-gpu`, with Docker permission to pass
`/dev/dri` into the container, then manually dispatch the workflow. Do not use a
general-purpose personal workstation or a runner holding unrelated secrets.

This will establish Arch userspace ABI, packaging, activation, nested
compositor, and configuration-load evidence only after a passing run is
recorded here. It will exercise a DRM render node but will not prove broad GPU
compatibility, physical displays or input devices, compositor security, an Arch
kernel, installer, ArchISO, upgrade/rollback, distribution packaging, or release
readiness. Escape focus/cancellation, fixed approve-once execution, no-touch
graphical expiry, and fail-closed service-loss presentation are now exercised on
the ARM64 experiment; other close paths, broader keyboard-only behavior,
assistive technology, durable audit recovery, and the x86-64 evidence remain
separate gates until explicitly exercised.
