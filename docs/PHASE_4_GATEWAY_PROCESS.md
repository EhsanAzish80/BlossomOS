# Phase 4 synthetic-only gateway process checkpoint

Status: historical scaffold checkpoint plus the retained debug-only synthetic
fixture. The later feature-gated listener is documented in
`PHASE_4_PRODUCTION_LISTENER.md`; neither checkpoint is the Phase 4 exit baseline.

## Production behavior

`blossom-model-gateway` is a workspace binary matching the fixed systemd unit
path. Default builds return a content-free not-ready category and exit code 78
before binding or connecting any socket. Release builds contain no synthetic
fixture entry point. A non-default package feature now compiles the production
listener path, but packages must not enable it until installed evidence passes.

Unknown arguments fail closed. There is no endpoint, socket, provider, model,
prompt, identity, mount, environment, resource, tool, or lifecycle option in the
production invocation.

## Synthetic process evidence

Debug Unix builds expose one explicit `--synthetic-fixture` mode for repository
tests. Its configuration is accepted only from test-owned environment variables,
cannot target the fixed production socket, refuses an existing path, sets the
ephemeral socket to `0600`, serves one connection, and removes the socket.

The only constructible cross-crate request is
`fixed_synthetic_gateway_request()`. It accepts no arguments and fixes the
developer-authored prompt, llama.cpp fixture provider/profile, empty intent
catalogue, text-only output, request ID, and deadline. It cannot represent
private or ambient input.

Target-Linux integration tests launch the actual gateway binary as a separate
process, authenticate real `SO_PEERCRED` values, complete the fixed canonical
request, and reject a wrong client identity before hello/request processing.
The pre-existing protocol tests continue to prove that a wrong gateway identity
causes the client to send zero request bytes.

## Logging and errors

Process errors use a four-value, content-free taxonomy. The binary prints only a
fixed category sentence: never a socket path, UID/GID, prompt, completion,
request payload, provider error, OS error, or credential structure.

## Deliberately absent

- an enabled-by-default production listener or installed listener evidence;
- installed-profile registry, manifest/artifact/unit validation, or readiness;
- an Ollama or llama.cpp connection, provider lifecycle, namespace observation,
  model binary, model file, download, or GPU device;
- private input, arbitrary synthetic prompts, tool proposals, broker access,
  shell, sudo, or privileged-helper access; and
- service installation, activation, or a claim that ADR-0012 is complete.

## Next checkpoint

The debug/test-only closed synthetic registry and deterministic unit renderer are
now documented in `PHASE_4_PROFILE_RENDERER.md`. Next, define root-owned account
resolution and complete artifact/manifest/unit validation as one runtime
readiness transaction, without opening the production listener or permitting
private input.
