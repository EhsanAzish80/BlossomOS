# Screen-lock installed-package evidence

Status: complete for the ARM64 installed-package boundary described below on
2026-10-02. The commit containing this record is the authoritative source
state. This was a local package gate; no full image or GitHub-hosted build ran.

## Environment and installation boundary

- Architecture: `aarch64`
- Guest resources: 4 vCPUs, 8 GiB RAM
- Disk: disposable qcow2 overlay on the existing ARM64 development image
- Package: `blossom-shell 0.9.0-7`, built inside the VM and installed with
  `pacman -U`
- Package SHA-256:
  `2a861aa8a21aeba57a157e2eef72c06151a236b9d0ae98dd2e0adb36fbf2e759`
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

## Automated checks

- shell packaging source check: passed
- shell QML source check: passed
- shell client-plugin source check: passed
- desktop-launcher source check: passed
- `git diff --check`: passed
- installed package and PAM/service contents: checked after `pacman -U`

## Limitations

- This is installed-package evidence in an ARM64 VM, not a clean-image gate or
  physical Intel MacBook evidence.
- The growing delay is session-local and survives a lock-client restart. A
  logout or reboot resets it; it is an online guessing throttle, not an
  account-level lockout.
- The manual portion is one tester's observation, not a usability study.
