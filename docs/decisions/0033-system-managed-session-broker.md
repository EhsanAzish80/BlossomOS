# ADR-0033: System-managed per-user shell broker

Status: Accepted

## Context

The shell broker owns `org.blossomos.Shell1` on the user's session bus and must
authenticate the private model gateway with `SO_PEERCRED`.  Running the broker
as a hardened user unit made that authentication unreliable.  Mount namespace
hardening such as `ProtectSystem`, `ProtectHome`, and `PrivateTmp` causes the
unprivileged user manager to create a user namespace.  That namespace maps only
the desktop user's UID; root and the gateway account appear as the overflow
identity.  The broker therefore cannot distinguish the gateway's real UID.

`PrivateUsers=identity` is not a remedy in a user unit because an unprivileged
user manager cannot construct the required identity mapping.  Weakening the
gateway identity check would turn a packaging error into an authentication
bypass.

## Decision

The system manager starts `blossom-shell-broker@1000.service`.  The template:

- binds its lifetime to `user@%i.service`;
- constructs the existing mount and process sandbox as the system manager;
- drops to `User=%i` before executing the broker;
- joins the user's existing session bus through
  `DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/%i/bus`;
- grants only the reviewed supplementary `blossom-ai` group needed to reach the
  private gateway socket; and
- preserves the existing resource, syscall, capability, address-family, and
  filesystem restrictions.

The package installs a system-manager wants link for the fixed Blossom desktop
UID 1000.  This is the lifecycle hook: login starts `user@1000.service`, which
starts the broker, and `BindsTo=` stops the broker with that user manager.  The
desktop session continues to start only the presentation units.  Session D-Bus
activation for the broker is not installed, because it would recreate the
user-manager namespace path.

The template rejects every instance other than 1000. Its home and runtime
environment are explicit rather than derived from the system manager's `%h`
specifier, which denotes root's home while the manager constructs this unit.

The broker still runs as the desktop user, never as root or as the gateway
account.  Its D-Bus name, methods, session-bus peer authentication, approval
boundary, and public shell interface are unchanged.

## Security meaning

With the system manager constructing the sandbox without a user namespace,
`SO_PEERCRED` again exposes host identities.  The broker must observe the
gateway's configured UID (currently 963), while the gateway must observe the
actual desktop UID.  Either mismatch fails closed.

Qualification must verify all of the following:

1. `/proc/$BROKER_PID/uid_map` is the host identity map, not a one-UID map.
2. `systemd-analyze security blossom-shell-broker@1000.service` completes and
   the reviewed hardening directives remain present.
3. A missing gateway and a socket owned by an unexpected user both fail closed
   with explicit activity reasons.
4. The installed package contains no session D-Bus activation fallback for
   `org.blossomos.Shell1`.

## Consequences

The broker is system-managed but gains no system authority.  The fixed UID link
matches the installer and live-image account contract; supporting arbitrary
desktop UIDs later requires an explicit lifecycle generator or PAM hook.  A
failure of `user@1000.service` also stops the broker, which is intentional.

Rollback consists of removing the wants link and system template.  Restoring a
hardened user unit is not permitted unless peer identity remains meaningful and
is demonstrated by the same runtime checks.
