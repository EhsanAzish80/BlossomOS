# Screen-lock installed-package evidence

Status: complete for the ARM64 installed-package boundary described below on
2026-10-02. The commit containing this record is the authoritative source
state. This was a local package gate; no full image or GitHub-hosted build ran.

## Environment and installation boundary

- Architecture: `aarch64`
- Guest resources: 4 vCPUs, 8 GiB RAM
- Disk: disposable qcow2 overlay on the existing ARM64 development image
- Package: `blossom-shell 0.9.0-8`, built inside the VM and installed with
  `pacman -U`
- Package SHA-256:
  `7a3770331b3ecc088a4c0bc35f094c92e8760b907de08b65d18ca8ff3de133a1`
- Authentication: a random disposable password enrolled only in the overlay;
  the password is not recorded in this evidence

The installed lock PAM policy contains only `pam_unix`. It does not use
`pam_faillock` and therefore cannot make the owner's local account unavailable
after a small number of screen-unlock mistakes.

## Runtime boundary

The dedicated lock service was checked from `/proc` before the manual test:

- `NoNewPrivs: 0`
- UID map: `0 0 4294967295` (host identity map)
- service declaration: `NoNewPrivileges=no`
- the main desktop shell retains its separate `NoNewPrivileges=yes` boundary

The first attempted implementation still showed `NoNewPrivs: 1` despite the
declarative `no` setting. The cause was systemd implicitly enabling the kernel
bit for seccomp-backed user-service restrictions. Those settings now appear on
a source-checked forbidden list for this PAM-owning service.

The source check also requires the remaining narrow boundaries in the other
direction: an exact `/usr/bin/quickshell` command and
`/usr/share/blossom-os/lock` QML path, `UMask=0077`, the private runtime
directory and restart policy. The lock unit cannot load QML from the workspace
or the user's home. The removed mount-namespace and seccomp-backed options are
listed in ADR-0038 with the PAM failure they caused; they remain forbidden for
this one PAM-owning user unit.

## Manual authentication cases

A single tester observed the installed lock UI and entered credentials using a
real keyboard:

| Case | Result |
| --- | --- |
| First incorrect password | rejected; local retry delay applied |
| Second incorrect password | rejected; longer local retry delay applied |
| Correct password after both failures | authenticated and unlocked |

The journal records two `pam_unix` failures followed by one successful PAM
authentication. The lock-local delays grow through 1, 2, 4, 8 and 16 seconds,
then remain capped at 30 seconds. A correct password cleared the state and
unlocked after two failures. The counter and monotonic deadline persisted
across a deliberate lock-client restart, then reset only after successful
authentication. They never alter login eligibility.

A separate fault-injection case made PAM's privileged checker unavailable.
The UI reported that it could not check the password, remained locked, and
left the incorrect-password counter at zero.

## Lock-client crash

While the session was locked, the qualification deliberately sent `SIGKILL`
to the lock client's main process. The tester observed only labwc's black
locked fallback; the desktop never appeared. systemd restarted the client with
a new PID, the client reacquired the locked session, and the password surface
returned. The active retry deadline survived that restart.

## Pending-effect cancellation

The tester opened the command bar, submitted
`create lock-cancel-proof.txt containing approval cancelled by locking`, and
waited for its exact approval preview. Super+L closed the approval and acquired
the session lock. After authenticating, the workspace file was absent and the
lock service had exited cleanly. No proposed effect survived behind the lock.

## Idle and explicit lock

A test-only 30-second swayidle timeout replaced the five-minute product value
without changing the installed configuration. With no inhibitor, the lock
service started exactly 30 seconds after swayidle. During a generated local
video playing fullscreen in Firefox, the same timer remained active for more
than 40 seconds and the lock service stayed inactive. Command+L over that
fullscreen video started the lock immediately at `13:09:47`.

## Suspend and lid ordering

The first suspend attempt found a real failure: swayidle 1.9.0 held a delay
inhibitor but did not execute `before-sleep`, while emitting its known
`BlockInhibited` parsing error. The gate did not count that attempt as a pass.
Revision 8 moved sleep ordering to a system-manager transaction unit. It starts
the existing user lock and waits for Quickshell's compositor-confirmed
`WlSessionLock.secure` state before allowing `sleep.target` to continue.

The real suspend transaction recorded these journal timestamps:

| Event | Timestamp |
| --- | --- |
| secure-lock unit started | `1790947475.123096` |
| secure-lock confirmation finished | `1790947475.268545` |
| sleep target reached | `1790947475.269097` |
| kernel entered s2idle | `1790947475.283691` |

For lid qualification, a test-only `python-evdev` uinput device exposed a real
`SW_LID` switch. logind attached it to seat0 and recorded `Lid closed`. The
ordered timestamps were:

| Event | Timestamp |
| --- | --- |
| logind received lid close | `1790947885.222994` |
| secure-lock unit started | `1790947885.260687` |
| secure-lock confirmation finished | `1790947885.477779` |
| sleep target reached | `1790947885.478063` |
| kernel entered s2idle | `1790947885.491952` |

The QEMU ARM `virt` machine enters s2idle but reports that guest wake-up is not
supported, including through QMP `system_wakeup`. Each ordering test therefore
used QMP reset after the journal had persisted. Resume remains a clean-image
and physical-hardware gate; lock-before-sleep and lid ordering passed here.

A runtime-only fault injection then masked `blossom-lock.service` and requested
suspend. The pre-sleep unit failed at `1790948090.721972`; `sleep.target` and
`systemd-suspend.service` both failed with `dependency`, logind completed the
operation without sleeping, and QMP reported the VM still `running`. The mask
was removed immediately afterward. A missing or failed lock therefore blocks
suspend instead of continuing unlocked.

## Automated checks

- shell packaging source check: passed
- shell QML source check: passed
- shell client-plugin source check: passed
- desktop-launcher source check: passed
- `git diff --check`: passed
- installed package and PAM/service contents: checked after `pacman -U`
- stale-process tripwire after service restart: passed with no Blossom
  `/proc/<pid>/exe` ending in `(deleted)`

## Limitations

- This is installed-package evidence in an ARM64 VM, not a clean-image gate or
  physical Intel MacBook evidence.
- The growing delay is session-local and survives a lock-client restart. A
  logout or reboot resets it; it is an online guessing throttle, not an
  account-level lockout.
- The manual portion is one tester's observation, not a usability study.
- This ARM QEMU machine cannot qualify resume-from-suspend; it can qualify the
  ordering up to kernel suspend, and physical hardware must qualify resume.
