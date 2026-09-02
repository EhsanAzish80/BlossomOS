# Phase 2 `services.read:status` evidence

Status: implemented and locally verified on 2026-09-02. Target-Linux evidence
is provided by the protected Quality workflow before merge.

## Contract

- Tool and capability: dedicated `services.read.status` and
  `services.read:status`, never a generic D-Bus or command request.
- Scope: one conservative exact `.service` name on the local system manager.
- Policy: explicit `ask`; only interactive `Approve once` reaches D-Bus.
- Result: requested and canonical unit, load state, active state, sub-state,
  system scope, and fixed provenance only.
- Semantics: a bounded observation that may change immediately; not an atomic
  snapshot, health verdict, enablement result, or security guarantee.

## Fixed native provider

The Linux provider connects to the fixed local
`unix:path=/run/dbus/system_bus_socket` address with a three-second deadline
around the complete D-Bus operation and a bounded queue. It sends `NoAutoStart`
calls only to `org.freedesktop.systemd1`:

1. `GetUnit(exact_approved_name)` on the fixed manager object and interface.
2. `Get` for `Id`, `LoadState`, `ActiveState`, and `SubState` on the returned
   unit object using the generic Unit interface.

It never calls `LoadUnit`, listing methods, `GetAll`, subscription or mutation
methods. It does not launch `systemctl`, a shell, the Phase 1 executor, or any
other process. Callers cannot select the connection address, destination,
object path, interface, method, property set, timeout, or credentials.

## Dependency review

The Linux-only implementation pins `zbus` 5.19.0 in `Cargo.lock`, with only its
`async-io` and blocking-client features. The already-transitive `async-io` and
`futures-lite` crates are declared directly so Blossom can apply one deadline
around the complete `NoAutoStart` operation. This is a maintained pure-Rust
D-Bus stack under the MIT license, compatible with Blossom's Apache-2.0
license. It replaces a substantially larger and less reviewable choice between
manual D-Bus wire-protocol code, C FFI, or parsing `systemctl` output.

Trust impact is limited to the unprivileged local D-Bus read path. The library
cannot choose the bus address or remote surface: Blossom supplies fixed values,
disables property caching and destination auto-start for method calls, bounds
total operation time and queue capacity, validates every returned field, and
provides no network or mutation API to callers. Dependabot, CodeQL, and protected CI cover
the locked dependency graph; future advisories require replacement, isolation,
or a reviewed upgrade under `docs/DEPENDENCY_POLICY.md`.

## Privacy and verification

The approval preview shows the exact unit and complete fixed operation. The
current result is terminal-safe because accepted units and states use bounded
ASCII token schemas. Persistent audit stores only domain-separated unit-name
digests, system scope, fixed provider identity, transitions, errors,
verification, and hash-chain identifiers; clear names and state values are
omitted.

Verification binds the approved requested unit, validates the canonical unit,
state bounds, system scope, and fixed destination/interfaces without making a
second I/O observation.

## Tests

Portable tests cover strict request/result schemas, conservative unit grammar,
unknown-field rejection, opaque future state tokens, exact approval binding and
replay rejection, exact preview, denial, cancellation, expiry, non-TTY refusal,
zero provider calls before approval, zero executor calls, verification, and
audit redaction.

Target-Linux review compiles the real zbus provider and exercises it against a
private `dbus-daemon` with a controlled minimal systemd-compatible service,
including exact success, unavailable-unit, whole-operation timeout, and missing
bus behavior. A real-systemd smoke observation is environment-conditional: a
runner without a usable system manager is skipped rather than converted into
false success.

This checkpoint adds no service enumeration, unit loading, mutation, logs,
process detail, generic IPC, Bash, sudo, LLM, privileged helper, or graphical
shell.
