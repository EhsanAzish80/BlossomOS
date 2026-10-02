# Screen-lock installed-package evidence

Status: complete for the ARM64 installed-package boundary described below on
2026-10-02. The commit containing this record is the authoritative source
state. This was a local package gate; no full image or GitHub-hosted build ran.

## Environment and installation boundary

- Architecture: `aarch64`
- Guest resources: 4 vCPUs, 8 GiB RAM
- Disk: disposable qcow2 overlay on the existing ARM64 development image
- Package: `blossom-shell 0.9.0-6`, built inside the VM and installed with
  `pacman -U`
- Package SHA-256:
  `016223d71aeca8e96646da03cdc3770107d4b9def540536349a7dc28816b41a2`
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
then remain capped at 30 seconds. They reset for a later lock session and never
alter login eligibility.

## Lock-client crash

While the session was locked, the qualification deliberately sent `SIGKILL`
to the lock client's main process. systemd recorded `Result=signal` and left
the service failed. The tester observed only labwc's black locked fallback;
the desktop did not reappear and no interaction was available. The VM was then
rebooted to recover the disposable test session.

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
- The growing delay is process-local. A process restart resets it; it is an
  online guessing throttle, not an account-level lockout.
- The manual portion is one tester's observation, not a usability study.
