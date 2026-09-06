# Narrow Blossom QML client plugin

This Qt 6 QML module is an untrusted transport adapter for the fixed Phase 6
surface. It exposes only start, approve-once, deny, pre-start cancellation, and
bounded activity refresh. Every D-Bus name, path, interface, method, protocol
version, activity limit, and request shape is compiled into the plugin.

The plugin cannot choose an executable, argument, capability, scope, path,
plan, provider, privileged operation, or D-Bus destination. It contains no
process launcher, shell, filesystem API, network API, approval token, policy,
executor, verifier, or audit writer. A malformed, oversized, unknown, or failed
reply clears the preview and becomes `unavailable`.

QML receives the exact service-authored preview and redacted activity only.
The same build produces `blossom-shell-ui`, a standard Qt application host that
loads only the fixed installed QML entrypoint. The host adds no callable API or
authority. It exists because the pinned Quickshell release does not publish its
QML child controls through Linux AT-SPI; the standard Qt host does. Quickshell
remains a separate, pinned, independently load-tested desktop presentation
process.
