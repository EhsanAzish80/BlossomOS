# ADR-0038: Local authentication lockout and recovery

- Status: Proposed
- Date: 2026-10-02

## Context

Blossom uses the local account password for login, screen unlock and approval.
Screen unlock must resist repeated guessing without making an offline,
single-user machine unavailable to its owner through an account lockout. The desktop shell also runs with
`NoNewPrivileges=yes`; PAM authentication must not require weakening that
general-purpose process.

## Decision

1. Screen unlock uses `pam_unix` for authentication but does not use
   `pam_faillock`. Failed attempts remain local to the active lock screen and
   impose delays of 1, 2, 4, 8, 16 and then at most 30 seconds. The password
   field and unlock action remain disabled for the whole delay. The attempt
   count and monotonic deadline live in the service's private runtime
   directory, survive a lock-client restart, and disappear at logout or boot.
2. The UI reports the remaining delay without revealing any account detail.
   A successful authentication clears the retry state and ends the locked
   session. Starting a later lock begins with a fresh retry counter; it never
   changes the owner's account availability. A PAM system error does not count
   as an incorrect password and never unlocks the session.
3. Screen authentication runs in a separate, minimal user service. The main
   desktop shell retains `NoNewPrivileges=yes`. The lock service is the only
   desktop component allowed to invoke PAM's privileged password checker.
   Because mount-sandbox directives in an unprivileged user manager create a
   UID namespace that maps root to the overflow user, this service deliberately
   avoids those directives. It also avoids seccomp-backed user-unit settings
   such as `LockPersonality`, `RestrictAddressFamilies` and
   `SystemCallArchitectures`: systemd implements them with
   `PR_SET_NO_NEW_PRIVS`, which prevents the setuid password checker from
   acquiring its required effective privilege even when the unit explicitly
   says `NoNewPrivileges=no`. The service instead has a closed executable and
   QML set and a private umask. Moving it to a system-managed user service is
   the preferred future hardening option because the system manager can apply
   the sandbox before dropping to the user's identity.
4. The lock is established with `ext-session-lock-v1`. If the lock process
   crashes, the compositor remains locked rather than exposing the session.
   systemd restarts the lock client, which reacquires the already-locked
   session and presents the authentication surface again.
5. Starting a lock denies pending approvals and closes the command bar and
   transient menus before requesting the session lock. No pending effect is
   restored after unlock.
6. Idle locking uses swayidle and the compositor's idle-inhibit protocol.
   Sleep ordering does not use swayidle's `before-sleep` path: version 1.9.0
   has a known `BlockInhibited` parsing regression, and the installed-package
   gate proved it could allow suspend without locking. A system-manager unit
   required by `sleep.target` instead starts the user lock and waits for
   Quickshell's compositor-confirmed `WlSessionLock.secure` state. Failure to
   obtain that confirmation fails the required unit rather than declaring the
   session safe to sleep.
7. Recovery-required lockdown is deferred. It
   must not ship until Blossom has an offline recovery mechanism generated
   during setup, stored separately, and qualified against denial of service.

## Planned recovery direction

The preferred design is a one-time offline recovery code generated during
setup and shown once for printing or storage away from the device. Recovery
must reset authentication state without disclosing or bypassing disk
encryption. Installer-media recovery based only on physical possession is not
accepted without a separate threat-model review.

## Consequences

- A guessing process is slowed while the session is locked, but cannot create
  a permanent account lockout. Restarting the lock process does not reset the
  delay; restarting the user session or machine does. This is an
  online-throttling measure, not an account-level defence.
- The narrow lock process has more privilege than the shell UI, so its package,
  QML imports and service command remain closed and source-checked.
- A gate must confirm the lock process has the host identity UID map; a
  single-UID remap would silently break PAM's privileged helper again.
- The same gate must confirm `NoNewPrivs: 0` in `/proc/<pid>/status`; the
  declarative systemd property alone does not reveal implicit activation by
  seccomp-backed user-unit settings.
- Approval authentication keeps its existing, separately measured challenge
  rate limit; this ADR does not weaken or merge that boundary.

## Qualification

- Correct password unlocks through real PAM.
- Consecutive wrong passwords impose the documented growing delays without
  changing the account's login state.
- Authentication succeeds with the correct password after a delay.
- Killing the lock UI leaves the compositor locked.
- The lock service restarts after a crash and restores the password surface
  without resetting an active delay.
- A PAM system error keeps the session locked and does not increment the
  incorrect-password counter.
- Locking while an approval is pending denies it and produces no effect.
- The main shell service still has `NoNewPrivileges=yes`.
- No recovery-required state is reachable in the shipped v1 policy.
