# Phase 4 production-listener checkpoint

Status: implemented behind the non-default `production-private-inference`
package feature. This is not installed-service evidence and does not complete
Phase 4.

The target-Linux path now keeps installed readiness descriptors alive, verifies
the gateway effective UID/GID, derives the logical model only from canonical
profile schema v5, reads and digests the boot ID once, and creates a fresh
kernel-random process nonce. It refuses any pre-existing fixed socket path,
changes only the socket group to the resolved `blossom-ai` GID, sets mode
`0660`, and verifies type, owner, group, mode, device and inode.

For every accepted descriptor it obtains `SO_PEERCRED` and authorizes the peer
against the retained account snapshot before sending hello or reading request
bytes. An authorized connection runs exactly one authority-free private frame
through the fixed llama.cpp adapter and the existing concurrent cancellation
state machine. Connections are processed sequentially, bounding active
inference to one. Per-connection failure does not widen authority.

The socket guard removes only the same socket device/inode on orderly unwind.
Systemd's non-preserved runtime directory remains responsible for cleanup when
the process is terminated without Rust unwinding.

Default builds do not compile or start this listener. A package must opt into
the feature only after the remaining installed target-Linux tests prove socket
lifecycle, unauthorized zero-input behavior, namespace/provider loss,
filesystem and network denial, resource limits, audit behavior, and offline
real-model inference. No such package enablement or evidence is claimed here.
