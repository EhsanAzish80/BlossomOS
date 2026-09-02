# Privileged helper package layout

These files define the future Arch package payload; they are not installed by
the repository test suite.

| Source | Installed path | Owner and mode |
| --- | --- | --- |
| release binary | `/usr/lib/blossom-os/blossom-privileged-helper` | `root:root 0755` |
| `blossom-privileged-helper.service` | `/usr/lib/systemd/system/blossom-privileged-helper.service` | `root:root 0644` |
| `org.blossomos.Privileged1.service` | `/usr/share/dbus-1/system-services/org.blossomos.Privileged1.service` | `root:root 0644` |
| `org.blossomos.Privileged1.conf` | `/usr/share/dbus-1/system.d/org.blossomos.Privileged1.conf` | `root:root 0644` |
| `org.blossomos.privileged1.policy` | `/usr/share/polkit-1/actions/org.blossomos.privileged1.policy` | `root:root 0644` |

The systemd unit creates fresh boot-scoped `journal` and `audit` directories
under `/run/blossom-privileged`, owned by root with mode `0700`. The package
contains no setuid file, sudo configuration, polkit JavaScript rule, shell
wrapper, generic command service, or network listener.

Installation and real-root activation remain unreleased validation gates. A
future Arch `PKGBUILD` must reproduce this table and must not weaken the checked
service, bus, or polkit files.
