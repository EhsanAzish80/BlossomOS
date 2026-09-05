# Apple Silicon shell validation experiment

Status: ARM guest boot, visible Hyprland rendering and DRM/VirGL diagnostics,
and the bounded Blossom graphical workflow are partially verified. The x86-64
gate, complete interaction matrix, durable recovery, and independent exit audit
remain open. This is a development experiment, not an accepted distribution
target or Mac installer.

## Development lineage and scope

The experiment evaluated a third-party ARM64 development prototype solely to
test the feasibility of an Arch Linux ARM guest using patched QEMU with Apple
Hypervisor Framework, virtio GPU, VirGL, ANGLE, and Metal. Those components and
patches are research inputs, not Blossom runtime evidence or a distributable
Blossom VM. Plain Homebrew QEMU must not be assumed to provide the same graphics
path.

The production direction is a Blossom-owned VM definition, image builder,
launcher, configuration, and user experience. No third-party desktop
configuration, branding, launcher, or optional host integration may ship as
Blossom. Replace the experimental runtime lineage with independently reviewed
Blossom sources before proposing distribution. Do not change the closed
production package registry, capability set, approval rules, or existing x86-64
CI gate. Arch Linux ARM package availability and versions require separate
validation; an ARM64 success cannot satisfy an x86-64 package compatibility
claim.

## Host preflight, 2026-09-05

- Host: ARM64, macOS 27.0 build 26A5425a.
- Memory: 32 GiB; logical CPUs: 10; `kern.hv_support`: 1.
- Available filesystem space: approximately 112 GiB at inspection time.
- Swift and pkg-config are available.
- QEMU, Docker and Colima were initially absent. See the subsequent setup below.
- Host capability checks do not prove QEMU graphics or guest compatibility.

Proposed initial budget: 4 vCPUs, 8 GiB RAM, 30 GiB sparse guest disk.
Reserve additional space for downloads, image assembly and runtime builds;
recheck free space before allocation. These are proposed limits, not allocated
resources or validated minimum requirements. Do not enable a local model.

## Approved build-tool setup, 2026-09-05

Installed Homebrew Colima `0.10.3`, Lima `2.2.0`, Docker CLI `29.8.0`,
Python `3.13.15` and its mpdecimal `4.0.1` dependency. Homebrew warns that
pre-release macOS 27 is unsupported. No login service was enabled.

Created and booted the dedicated `blossom-builder` Colima profile using VZ,
4 CPUs and 8 GiB RAM. Colima uses a separate 20 GiB system disk in addition
to its 30 GiB growable container-data disk. Verified guest `aarch64` and Docker
server `29.5.2`. The builder was subsequently stopped to release memory.
Its generated configuration disables SSH-agent forwarding and host public-key
loading; `findmnt -t virtiofs,9p` returned no mounts. Automatic guest TCP port
forwarding and default Docker-context activation were disabled. Local Docker
and containerd management sockets remain deliberately forwarded for building;
this is not a claim of an air-gapped or host-inaccessible guest.

Reference source checkout (ignored local build storage):
`build/work/arm64-vm-reference`, pinned at
`5d0edf616232b4be5f5ada9f14b10e017c9a8e36`.
The runtime build selects QEMU `c3d48b7d1e89604920e5b81b91140c2ad39a1943`
and retains upstream checksum checks. Its first attempt passed download checks
but failed configuration with Apple's Python 3.9 (`no usable tomli`). The retry
uses Python 3.13 through a command-local PATH, not a global shell change.

The retry succeeded: QEMU `11.1.1`, 16 self-contained Mach-O runtime images,
upstream capability checks, ad-hoc signatures and macOS 15.0 compatibility
validation passed. The relocated executable also returned its version after
moving into ignored build storage. Its SHA-256 is
`1f9469746c53992aab0afa7dbeb294be5dfeb9148b57afa969e49338f0022058`.
Runtime location: `build/work/arm64-vm-reference/macos/.build/qemu-gpu-runtime`.
This is a local development build, not notarized distribution or graphical
runtime evidence. Approximately 109 GiB remained free after setup.

The upstream guest assembly wrapper expects host bind mounts; it has not been run.
Before image assembly, adapt source/artifact transfer to explicit copies and
VM-local volumes so host-folder sharing remains off. Do not enable broad
mounts merely to make the reference build script work. Third-party notices
must accompany any later redistribution; Blossom's Apache license does not
relicense the bundled QEMU or guest dependencies.

## Desktop assembly attempt, 2026-09-05

Status: blocked before package installation; no desktop disk or boot evidence.
The reference container builder was attempted using a streamed `git archive`
of the pinned guest source, not a host bind mount. Its Alpine base resolved to
`sha256:48b0309ca019d89d40f670aa1bc06e426dc0931948452e8491e3d65087abc07d`.
The legacy Docker builder required explicit `TARGETARCH=arm64`.

The Arch bootstrap completed, but the full reference builder could not obtain
`rust=1:1.98.0-1`. No reference pin was changed. A separate minimal diagnostic
builder reused the completed Arch layer
`sha256:1a29e321b98bf8b1246dc0b1effa9f75d7d660484cff9bb8a17721a3558af1c0`
without Rust or desktop overlays. Although container bootstrap hooks reported
systemd errors, subsequent `pacman -Qk systemd dbus-broker` reported zero missing
files. This alone does not establish boot correctness.

The diagnostic container used only named Docker volumes for `/work` and
`/output`; `docker inspect` confirmed no bind mounts. It retained required
package signatures and did not add any optional-trust third-party repository.
Package resolution failed:

| ARM package | Resolved version | ABI relationship |
| --- | --- | --- |
| hyprland | 0.56.1-3 | requires libaquamarine.so=13-64 |
| hyprtoolkit | 0.5.4-5 | requires libaquamarine.so=13-64 |
| aquamarine | 0.15.0-2 | provides libaquamarine.so=14-64 |
| quickshell | 0.3.1-1 | available; not installed or load-tested |

Refreshing the database reproduced the failure. The California official
mirror independently advertised the same Hyprland/Aquamarine versions; the
German mirror's HTTPS certificate did not match its hostname, and validation
was not bypassed. Do not force installation, fake SONAME symlinks, skip
dependencies or count this as an installed shell pass.

Local evidence and draft diagnostic scripts are under ignored
`build/work/arm64-graphics/`: `build.log`, `artifacts/package-candidates.txt`,
`Containerfile`, `build-rootfs.sh`, `overlay/` and `launch.sh`. Scripts pass
ShellCheck and Bash syntax checks but have not completed an end-to-end run.
The draft launch disables network devices, host mounts, media/clipboard bridges
and full keyboard capture. Its locked, console-autologin test account and
8 GiB diagnostic factory disk are experiment-only, not product defaults.
No launcher was executed. The stopped failed container and VM-local volumes
are retained for diagnosis, not silently deleted.

Next decision: review a coherent signed ARM package snapshot, or explicitly
review source builds of the affected dependency set. Neither changes the
accepted x86-64 registry. The reference's old Rust pin must also be resolved
before claiming its full factory builder is reproducible today.

## Candidate snapshot review, 2026-09-05

The owner approved snapshot research. The community-operated archive at
`https://pkgmirror.sametimetomorrow.net/aarch64/repos/2026/09/03/`
provides a candidate; it is not an official Arch Linux ARM archive or a newly
trusted signing authority. An empty target database resolved the complete
279-package diagnostic transaction using only that date's core/extra/alarm/aur
repositories, with no current-mirror fallback. Key versions are:

- Hyprland `0.56.1-3`, Aquamarine `0.14.0-2`, hyprtoolkit `0.5.4-4`.
- Quickshell `0.3.1-1`, Qt base `6.11.2-3`, linux-aarch64 `7.2.2-2`.

Downloaded Hyprland and Aquamarine archives and detached signatures validated
with full trust against the existing Arch Linux ARM keyring, fingerprint
`68B3537F39A313B3E574D06777193F152BDBE6A6`. No archive-specific key was imported.
Their SHA-256 values match the snapshot metadata:

- Aquamarine: `a6930011012be9faa7342bf767d66504144e6fff67dad938ede8628cfbd716ae`.
- Hyprland: `4fcb1b5efe019e184a85b234f75151e68fd8f60ace9b06ff59e7ffbd8a280f7a`.

Reviewed database SHA-256 values:

- core: `6004e1d594eb582b85e0ebe68d5961a9ac5f1afc33208b4107ff3fe3c442ee3f`.
- extra: `ca5f2ff636c00c7472ab655b5ca39dae545c3b9ee955bb122cdcbcef27a40c28`.
- alarm: `018ba58799025a4a19f78de7c2a529ef1add611d2b2b88206bc3f9d841be4261`.
- aur: `bdd038c7c32dc6de8977c65659249f3ca4b498c303ded46750eb6ab6f1dad187`.

The locally retained `snapshot-review/resolved.txt` has SHA-256
`2dad904b012c022574b84451ce9a9ec40db31b71111cfe4fa9fbe1beb071cd4b`.
The config, databases and signature log are retained beside it under
`build/work/arm64-graphics/`. Metadata is not independently authenticated by
these locally measured hashes. Package signatures remain required; database
signatures remain optional as in the existing ARM bootstrap policy.

This resolves the dependency-search gate only. Before a build, require the
reviewed database hashes and exact transaction plan, authenticate every package,
and fail on missing archives, signatures, changed resolution or installation
errors. The remaining 277 packages have not been individually authenticated in
this review. No snapshot packages were installed, no new desktop image was
created, and the Rust pin and production x86-64 registry were not changed.

## Authenticated image assembly, 2026-09-05

The owner approved full package verification and a diagnostic image build.
The 279-package transaction completed with required package signatures and
integrity verification against the established keyring. Installation used
copies of the reviewed databases and authenticated archives in a VM-local
file repository: no mutable mirror refresh or host bind mount was used.
The installed name/version list exactly matches the reviewed transaction.

Rate limiting interrupted both parallel and single-stream pacman downloads.
A bounded fetch then waited 60 seconds and spaced requests by five seconds,
retaining successful downloads and respecting delayed retries. Three entries
in the original reviewed plan were builder-local cached packages (diffutils,
mkinitcpio and mkinitcpio-busybox), not network URLs; these were copied only
from their exact cache paths and subjected to the same subsequent checks.
This clarifies the earlier all-snapshot-source wording: version resolution
was from the frozen snapshot, but those three bytes came from the builder cache.

Detached-signature requests also encountered throttling. All 279 embedded
PGP signatures were extracted from the hash-checked snapshot metadata and
supplied locally to pacman's normal verifier. No new signing authority or
weakened trust option was added. An unrelated all-NUL `findnewest-0.3-4/desc`
record was found in `extra.db`; it is not selected. Every selected package
had matching name/version/architecture/filename metadata and a signature.
This archive-quality finding prevents treating the entire repository as audited.

Verified during this build:

- 279 archive hashes recorded in `package-sha256.txt`.
- 279 installed package versions match `expected-packages.txt` exactly.
- `quickshell --version` loads inside the ARM guest root and reports `0.3.1`.
- Linux `7.2.2-2-aarch64-ARCH` initramfs generation succeeds with virtio modules.
- `e2fsck -fn` passes for the 8 GiB diagnostic filesystem.

The build reported chroot-not-a-mountpoint warnings and missing firmware for
unrelated physical GPU/storage drivers. Those are retained in the log, not
silenced. No physical-hardware compatibility is claimed. The diagnostic
configuration uses the pinned
[Hyprland 0.56.1 Lua API](https://github.com/hyprwm/Hyprland/blob/v0.56.1/example/hyprland.lua).

Artifacts are copied, not shared, to ignored local directory
`build/work/arm64-graphics/verified-image/`. Image manifest SHA-256 values:

- `Image`: `7274b843e925e6e470b16ffffc6acf33b8a9d30b8117e7f9ef7dcd32f2924789`.
- `initramfs-linux.img`: `31875ac97990107d349d1cf5b4487158d6255a03fdd11b48bc38160a0091fee0`.
- `rootfs.ext4`: `82dc0974a67cd7dfd2237e1d89f4b9b89a8940b13114069785a3a4dea416675e`.
- `packages.lock.txt`: `3d5e689aefb3d618f16c860de17b45b4cda1286c99cc558163f8d53818066b2f`.

All four exported manifest hashes passed verification on the Mac after copying.
The builder is stopped after export. These are development artifacts, not a
signed release, reproducible-build claim, installer or production ARM registry.
Next: separately build/install and test the Blossom slice.

### First diagnostic boot — September 5

The manifest passed again before launching a private writable clone. Serial
output reached `Arch Linux ARM 7.2.2-2-aarch64-ARCH (ttyAMA0)` and the login
prompt. The running QEMU arguments confirm 4 CPUs, 8 GiB RAM, `-nic none`,
and no host folder, clipboard or audio devices.

The host graphics log reports OpenGL ES 3.0 through ANGLE and
`ANGLE Metal Renderer: Apple M1 Max`. This proves backend initialization,
not that Hyprland successfully renders through it. The log also reports an
MSAA override to one sample; graphics correctness is not yet established.

Evidence: ignored `build/work/arm64-graphics/run/serial.log`, the launch
session output and the inspected running process arguments. Native UI discovery
did not expose the QEMU window; selecting its executable was blocked because it
could start a duplicate VM. Subsequent owner-supplied screenshots provide the
visual evidence below. The VM remains running for inspection.

### Owner-observed graphics checkpoint — September 5

Screenshots show the graphical terminal, successful keyboard input, and
`hyprctl systeminfo` reporting Hyprland 0.56.1, the DRM backend, GL 3.0 and a
1280 x 800 virtual display. `/dev/dri/card0` and `renderD128` exist. `eglinfo -B`
reports `virgl` for GBM and Wayland with OpenGL ES 3.0, supporting the intended
guest VirGL to host ANGLE/Metal path. This is not a performance, graphics
correctness, DMA-BUF interoperability or broad compatibility test.

Aquamarine build/runtime versions match at 0.14.0. Hyprutils reports built
against 0.14.0 but running 0.14.1; this is recorded as a compatibility follow-up,
not silently treated as an exact-match pass. Inspection of the authenticated
archives shows Hyprland requires `libhyprutils.so=13-64` and Hyprutils 0.14.1-1
provides that same ABI. This addresses declared ABI compatibility, not all
runtime behavior. No library downgrade or production registry change was made.
The screenshots are user-provided observations, not
machine-readable logs or an automated end-to-end test.

Blossom's service/plugin/QML were not added to the preserved original graphics
guest. They are installed only in disposable shell-test images described below.
Graphical preview, denial, Escape cancellation, approve-once verification,
no-touch expiry, and fail-closed service-loss presentation now pass there; the
remaining interaction and durable recovery gates stay open.

### ARM shell build and installed-library check — September 5

The isolated builder compiled the service from source revision
`427a2841066a39538693640fdaf0735287454d75` with Rust 1.98.0 and built the native
plugin against Qt 6.11.2. The build dependency manifest contains 221 packages
resolved from the same frozen snapshot; its SHA-256 is
`73b1d7ef72ec0769bddd099830c6b7800ed3089a51aa662d82d746d3a59cb501`.
The 44-package installation transaction passed keyring and integrity checks.
Build-root post-install hooks warned about absent `/proc` and `/dev/null`;
the subsequent compiler and test commands ran through arch-chroot. These
warnings are not evidence of a production installation pass.

Six feature-enabled session-service tests passed. An offline core run filtered
by `shell_` passed 20 tests (including one OS-identity parser test selected by
that substring). These cover replay, peer binding, expiry, cancellation and
service loss at the unit/protocol level, not graphical interaction.

The first installed ELF check found a real packaging defect: the QML loader
depended on `libblossom-shell-client-plugin.so`, which remained outside the
copied module directory. The local CMake fix places both libraries together
and sets their RUNPATH to `$ORIGIN`. A fresh offline ARM rebuild and dependency
check in a separate runtime root passed; both RUNPATHs were inspected. Static
and installed-CI checks now require this layout, but the changed CI has not
run remotely. No capability or service hardening was relaxed.

The separate shell-test image retains the original 279-package list exactly;
Rust, GCC, CMake, Ninja and sudo are absent from that runtime root. Only the
service, its unchanged activation/unit files, corrected plugin, existing QML
and diagnostic session configuration were added. The session requires the
systemd user bus and has no private-bus fallback. Filesystem checks passed.
Local artifacts are under `build/work/arm64-graphics/shell-image/`; the original
`verified-image/` and currently running graphics clone remain preserved.
All four exported manifest hashes passed on the Mac. The shell-test disk hash
is `b91acf03a099a6a4f9e71d2550b3c47f9d09abd44dd22b0ee7e045a5b612983a`;
kernel, initramfs and package-list hashes are unchanged from the graphics
image. The builder was stopped after export and verification.

The shell-test lineage has now booted under Hyprland and loaded the installed
Quickshell module. Runtime investigation found and corrected three integration
defects without weakening policy or sandboxing: the hardened user unit hid the
session bus, Qt encoded bounded `quint16` arguments with the wrong D-Bus wire
type, and the approval window did not assign active focus for its Escape key
handler. Explicit expiry reporting was also added so elapsed approvals are not
misreported as service unavailability.

Owner screenshots from corrected disposable ARM64 images verify the exact
preview, terminal denial, terminal Escape cancellation, and approve-once
verification. The cancellation
activity contains request, policy, approval, and terminal-cancelled records for
one request ID, with no execution or verification record. The focus-fix image
was assembled offline with the unchanged runtime package manifest; its ext4
filesystem check and exported SHA-256 manifest passed before boot. These are
real installed ARM64 observations.

The approve-once investigation found that systemd hardening initially denied
Bubblewrap's namespace setup (`AF_NETLINK`) and safe path resolution
(`openat2()`). After narrowly correcting those service constraints, an exact
production-profile probe isolated the remaining failure: `--disable-userns`
attempted to write `/proc/sys/user/max_user_namespaces`, which
`ProtectKernelTunables=yes` intentionally makes read-only. The option was
removed as redundant for this fixed trusted `/usr/bin/uname -s` executor. The
profile still unshares all namespaces including the user namespace, drops all
capabilities, clears the environment, exposes `/usr` read-only, and exposes no
network, procfs, devices, or writable temporary filesystem.

The final offline image passed six feature-enabled service tests, ext4 checking,
and all exported SHA-256 checks. Its root filesystem hash is
`36d540929b073e387c095406e0710e529ac736364acb09a4b75184583f1cd59b`.
The owner-observed graphical run reached `verified`; its seven correlated audit
records cover request, policy, approval issuance, one-time approval, execution
start, execution finish, and terminal verification for one request ID.
The subsequent no-touch test exposed a missing native-client deadline trigger.
A bounded one-shot timer now uses the fixed preview deadline to request
cancellation, while the service independently checks expiry and remains the
only authority that consumes the approval. The rebuilt image passed native
compilation, ext4 checking, and exported checksum verification; its root
filesystem hash is
`c83bdccffe5163ebe47234bf3044f969f282e96a679cbef4f94ce658cba9302b`.
Owner evidence shows the panel closing automatically with status `expired` and
terminal cancellation, without approval, execution, or verification records.
The first installed service-loss run then showed stale waiting state after the
fixed D-Bus owner disappeared. The native client now watches only
`org.blossomos.Shell1` for owner unregistration and fails closed by stopping its
timer, clearing the preview, closing the panel, and reporting `unavailable`.
A deterministic watcher used to terminate the broker is confined to the ignored
disposable image. The corrected plugin compiled with warnings as errors; ext4
and exported checksums passed, with root filesystem hash
`23883d2dd8979d0ee6c95931b5a07fdbd199f47d8f573ea89630d49f655613cb`.
Owner evidence shows the pending panel disappearing within the test interval
and status becoming `unavailable`. Because the deliberately terminated service
held its audit only in memory, the empty activity view does not establish
durable audit recovery. These observations still do not establish accessibility,
general hardware support, the x86-64 installed gate, or Phase 6 completion.

## Ordered gates

1. Pin and inspect the reference source revision, runtime patches, dependency
   provenance and licenses. Select a verified release or reproducible build;
   do not execute a remote installer or silently trust a mutable download.
2. Obtain owner confirmation for the concrete runtime/builder installation,
   download size and storage location before adding host software. Use a new
   dedicated disposable VM, never an existing personal guest.
3. Keep host folders, clipboard, camera, microphone, port forwarding and
   accessibility grants off. Do not forward SSH agents or expose host secrets.
   Inspect actual runtime arguments and bridges to verify these exclusions;
   an upstream default is not evidence. If the reference cannot disable them,
   stop and review a minimal configuration instead of launching it as-is.
4. Boot the ARM64 guest and record kernel, package versions, runtime revision,
   image hashes, renderer, DRM render node and DMA-BUF availability. Distinguish
   hardware acceleration from software rendering. Check graphics with actual
   Hyprland, not only an advertised QEMU feature or a device node.
5. Record a separate experimental ARM64 dependency manifest. Build Blossom's
   Rust service and native Qt plugin in the guest and install the existing
   fixed-path service, activation metadata, plugin and QML. Review architecture
   assumptions before using any x86-64 installation harness.
6. Run regression tests and the real graphical fixed-uname flow: exact preview,
   approve once, deny, cancellation, expiry, service loss, verification and
   correlated redacted activity. Include focus/close and keyboard-only checks.
   Record failures honestly; do not relax sandbox or policy to get a pass.
7. Review evidence before proposing any ARM64 support ADR or production lock.
   Keep the existing x86-64 installed gate and independent exit audit open.

## Future launcher, not part of this experiment

A self-contained Mac app could expose validated CPU, memory and disk choices,
with host headroom checks and explicit allocation confirmation. Storage limits
must distinguish maximum guest capacity from current host disk usage. Disk
shrink/reset and removal need separate destructive-action confirmation.
Host integration must remain opt-in and separately reviewed. Signing,
notarization, updates, rollback, bundled dependency obligations and guest
isolation need their own design and evidence before distribution.

The VM boundary supplements, and does not replace, Blossom's capability broker,
policy engine, one-use approval custody, sandbox executor and audit system.
