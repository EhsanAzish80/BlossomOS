# Blossom shell package boundary

Status: reviewed, inactive package inputs. These files are not installed or
enabled by cloning or testing the repository.

`../registry/arch-x86_64.lock.json` fixes the first supported package set to
official stable Arch x86-64 repositories observed on 2026-09-04. It is a
compatibility and evidence input, not permission to silently upgrade packages.
Any version change requires review, a lock update, and fresh installed evidence.
Testing repositories, AUR packages, moving branches, and version ranges are
rejected. Installed evidence must additionally use a dated repository snapshot
and verify packages through pacman's Arch keyring.

| Source | Installed path | Owner and mode |
| --- | --- | --- |
| release binary | `/usr/lib/blossom-os/blossom-shell-service` | `root:root 0755` |
| `blossom-shell-service.service` | `/usr/lib/systemd/user/blossom-shell-service.service` | `root:root 0644` |
| `org.blossomos.Shell1.service` | `/usr/share/dbus-1/services/org.blossomos.Shell1.service` | `root:root 0644` |

The service runs as the logged-in unprivileged user, owns only the fixed
session-bus name, and exposes the closed versioned Rust interface. The unit has
no shell, sudo, helper, network listener, caller-selected executable, generic
D-Bus target, or enablement target.

`RestrictNamespaces=` and a restrictive system-call allowlist are deliberately
deferred because the fixed Bubblewrap executor must create its code-owned
namespaces. Installed evidence must prove the combined systemd and Bubblewrap
boundary before additional hardening is accepted.

The unit permits `AF_NETLINK` only because Bubblewrap needs a `NETLINK_ROUTE`
socket while constructing its private network namespace. It does not permit
`AF_INET` or `AF_INET6`, and `IPAddressDeny=any` remains in force.
`RestrictSUIDSGID=` is also deliberately absent: current systemd implements it
by denying `openat2()`, while Bubblewrap uses `openat2()` for safe source-path
resolution. The service remains unprivileged, has an empty capability set,
cannot gain privileges, and sees read-only system and hidden home trees.

The fixed executor deliberately omits Bubblewrap's `--disable-userns`: that
option writes `/proc/sys/user/max_user_namespaces`, while
`ProtectKernelTunables=yes` makes the tunable read-only. The executor still
unshares all namespaces including the user namespace, drops all capabilities,
selects the fixed trusted `/usr/bin/uname -s` command itself, exposes `/usr`
read-only, and exposes no network, procfs, devices, or writable temporary
filesystem.

This checkpoint does not package QML, start Hyprland or Quickshell, establish a
graphical session, or claim installed compatibility.
