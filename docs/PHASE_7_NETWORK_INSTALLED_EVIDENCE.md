# Phase 7 installed network evidence

Status: complete for the fixed network-connectivity slice on 2026-09-07.

The authoritative evidence is GitHub Actions run
[`34126485600`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34126485600),
which passed against signed commit `a9f934c`.

## Real online boundary

Job
[`101756616945`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34126485600/job/101756616945)
ran the production provider directly on the dedicated x86-64 Ubuntu host against
its real root-owned NetworkManager service. It returned:

```text
network-evidence=online schema=1 source=system.network.connectivity
```

## Real isolated non-internet boundary

Job
[`101756617068`](https://github.com/EhsanAzish80/BlossomOS/actions/runs/34126485600/job/101756617068)
started a real NetworkManager daemon inside an isolated network namespace. The
namespace had no default route and its fixed loopback connectivity endpoint was
unreachable. NetworkManager's command-line view settled to `limited`; the
production D-Bus provider independently returned the closed `local` class:

```text
isolated-connectivity=limited
network-evidence=local schema=1 source=system.network.connectivity
```

Both are native non-internet states. The installed assertion accepts only the
closed `offline`, `limited`, or `local` classes and rejects `online`, `unknown`,
missing data, transport failure, stale data, and schema drift. Exact mapping of
all five classes remains covered by deterministic core tests.

## Evidence limits

This proves the fixed Linux NetworkManager adapter on one real online Intel host
and one isolated x86-64 namespace. It exposes no interface, address, route, DNS,
access-point, or traffic identity. It is not broad hardware, installer,
distribution-image, upgrade, rollback, or release-readiness evidence.

