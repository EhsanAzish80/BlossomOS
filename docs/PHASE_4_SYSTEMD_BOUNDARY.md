# Phase 4 inactive systemd packaging boundary

Status: implemented as reviewed package metadata and inactive templates only.
It is not installed runtime isolation or the Phase 4 exit baseline.

## Defined surface

- Two persistent, non-login, distinct system accounts are declared through
  `systemd-sysusers`: `blossom-model-gateway` and
  `blossom-model-provider`.
- The `blossom-ai` group is the opt-in inference connection boundary. It grants
  no broker, tool, file, shell, sudo, or privileged-helper authority.
- `/run/blossom-model-gateway/inference.sock` remains the only planned private
  client ingress. The future gateway must create it as
  `blossom-model-gateway:blossom-ai 0660` and authenticate both sides with
  kernel credentials.
- `blossom-model-netns.service` anchors a private loopback-only network
  namespace. The gateway and exactly one closed provider unit join it.
- Provider templates exist separately for Ollama and llama.cpp; there is no
  generic `%i` unit or caller-selected executable, endpoint, model, resource,
  environment, mount, namespace, or device.

Numeric service IDs are allocated collision-free by the target system and then
persist in root-owned account data. They are not `DynamicUser=` identities. The
future readiness implementation must resolve the fixed account names once,
reject root/shared/missing identities, and bind the resolved UID/GID to the
manifest, connected peer, unit, and runtime evidence without caller input.

## Hardening represented by the templates

All three service roles use a private network namespace, empty capability and
ambient-capability sets, `NoNewPrivileges`, private temporary and device views,
strict system protection, no home access, kernel/control-group restrictions,
address-family restrictions, syscall filtering, disabled core dumps, and
bounded memory, swap, CPU, tasks, file size, and open files. Providers receive
only `AF_INET` loopback and no Unix-socket or device access. Their only writable
path is profile-specific disposable state under `/var/lib/blossom/`. The wider
Blossom package tree is inaccessible and the selected binary/model are exposed
through exact read-only binds. Dynamic-loader and ordinary system-library paths
remain readable; this checkpoint does not claim a hermetic provider root.

Ollama's endpoint and model-store environment values are literal code-owned
unit data; the manifest schema allowlists their names but never accepts values
from callers or models. The future rendered unit digest binds those values.

## Validation

`scripts/ci/check_model_runtime_packaging.py` requires the exact package-file
surface, identities, dependency graph, paths, hardening directives, provider
templates, render-token vocabulary, lack of generic instances/devices, and
absence of `[Install]` sections. Linux CI also runs `systemd-analyze verify` on
the concrete namespace and gateway units. Provider templates cannot be systemd
verified until a later checkpoint renders reviewed artifact paths and resource
values; the checker validates their closed syntax and tokens meanwhile.

## Deliberately absent

No package recipe, production renderer, compiled profile registry, provider or
model artifact, installed manifest, account creation, socket creation, service
enablement/start, namespace execution, gateway process, provider process,
private input, or real-model inference is added. These files are not proof that
the runtime isolation works; adversarial production-path Linux evidence remains
mandatory before private bytes can be admitted.

## Next checkpoint

Implement the small gateway process around the already closed protocol and
synthetic provider adapters, still admitting synthetic data only. Package
rendering and runtime readiness must then bind actual identities, unit and
artifact digests, filesystem state, namespace membership, and provider health
before private input is considered.
