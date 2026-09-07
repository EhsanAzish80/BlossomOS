# Narrow Blossom QML client plugin

This Qt 6 QML module is an untrusted transport adapter for the fixed Phase 6
surface plus the fixed Phase 7 battery projection. It exposes only start,
approve-once, deny, pre-start cancellation, bounded activity refresh, and the
fixed battery refresh. Every D-Bus name, path, interface, method, protocol
version, activity limit, and request shape is compiled into the plugin.

The plugin cannot choose an executable, argument, capability, scope, path,
plan, provider, privileged operation, or D-Bus destination. It contains no
process launcher, shell, filesystem API, network API, approval token, policy,
executor, verifier, or audit writer. A malformed, oversized, unknown, or failed
reply fails closed. Battery replies additionally require the exact closed
schema and code-owned freshness bound, and are cleared on expiry or service
loss.

QML receives the exact service-authored preview, redacted activity, and narrow
battery display projection only.
The same build produces `blossom-shell-ui`, a standard Qt application host that
loads only the fixed installed QML entrypoint. The host adds no callable API or
authority. It exists because the pinned Quickshell release does not publish its
QML child controls through Linux AT-SPI; the standard Qt host does. Quickshell
remains a separate, pinned, independently load-tested desktop presentation
process.
