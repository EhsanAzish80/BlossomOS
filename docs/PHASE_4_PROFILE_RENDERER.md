# Phase 4 closed synthetic profile renderer checkpoint

Status: implemented for debug and test evidence only. This is not a production
profile registry, package renderer, or Phase 4 exit baseline.

## Closed registry shape

`fixed_synthetic_provider_package()` accepts only the closed `GatewayProfile`
enum. It has no path, digest, argument, environment, identity, resource, mount,
device, endpoint, model, or template input. For each of the two CPU fixture
profiles it fixes:

- provider kind and protocol versions;
- synthetic package-owned runtime/model paths and placeholder digests;
- exact executable arguments and environment-name allowlist;
- fixed loopback endpoint and inference path;
- exact read-only artifacts and one profile-specific disposable write path;
- no devices;
- bounded CPU, memory, swap, tasks, open files, output, and deadline values; and
- synthetic non-root distinct IDs and exact gateway/provider/namespace units.

The synthetic IDs and repeated placeholder artifact digests are visibly test
data. They are not accepted production identities or artifact evidence.

## Deterministic rendering

The renderer compiles the reviewed provider-specific `.service.in` files into
the core test build, replaces an exact token vocabulary with the corresponding
fixed fixture values, rejects missing or remaining profile tokens, and derives
`unit_sha256` from the exact rendered bytes before canonicalizing the manifest.
Tests render both profiles twice and require byte-identical units and manifests,
matching command lines, matching unit digests, no generic `%i`, no device rule,
and no unresolved provider/model tokens.

Ollama's fixed service command is `ollama serve`; its model is selected through
the already closed inference request while the package-owned model store,
loopback address, and environment names remain unit/profile owned. llama.cpp's
fixed command binds the selected synthetic model path directly.

## Linux CI evidence

The `render_synthetic_provider_units` example is a CI-only harness. It writes
both fixed rendered units into a caller-supplied existing absolute test
directory with create-new semantics. CI creates inert placeholder artifacts,
renders both units, and submits the namespace, gateway, Ollama, and llama.cpp
units together to `systemd-analyze verify`.

The harness is not installed or invoked by the gateway. It cannot choose values
inside a profile and does not start a service, namespace, provider, or model.

## Release boundary

The package type, closed factory, renderer, and CI harness depend on
`debug_assertions`. Release builds retain only the opaque
`ProviderProfileSpec`, with no constructor, and the gateway continues to exit
not-ready before opening a socket. Therefore this checkpoint cannot make a
manifest authoritative or admit any input.

## Deliberately absent

- trusted account-name resolution or installed numeric identity binding;
- root-owned artifact, manifest, or rendered-unit loading and digest checks as
  one readiness transaction;
- a production registry, renderer, package recipe, or signed artifact;
- listener creation, group authorization, provider startup, health checks, or
  namespace membership proof;
- private input, real prompts/models, downloads, GPU access, or tool execution.

## Next checkpoint

Implement root-owned account resolution and a fail-closed runtime readiness
validator that binds the installed manifest, runtime set, model, rendered unit,
resolved identities, expected namespace/unit names, and immutable descriptor
evidence. Keep production listening and private input disabled.
