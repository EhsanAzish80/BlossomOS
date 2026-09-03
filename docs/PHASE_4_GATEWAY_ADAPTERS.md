# Phase 4 gateway-to-adapter checkpoint

Status: implemented for synthetic debug/test evidence only. This is not a
production gateway, private-input path, provider package, or Phase 4 exit
baseline.

## Closed route

`serve_synthetic_gateway_via_adapter_once()` preserves the existing
client-to-gateway order:

1. accept one Unix connection;
2. validate kernel peer credentials before reading or writing request data;
3. send the bounded profile-bound hello;
4. decode exactly one synthetic-classified request;
5. select `OllamaAdapter` or `LlamaCppAdapter` from the closed
   `GatewayProfile` enum; and
6. re-encode only normalized, schema-validated adapter events into gateway
   frames through the same connection.

There is no caller-selected endpoint, path, provider kind, transport, model
artifact, environment, mount or adapter argument. Adapter and gateway bounds,
terminal-state validation and proposed-intent validation remain in force.

## Linux evidence

One end-to-end Linux test exercises both closed profiles sequentially. Each
case uses an authenticated Unix gateway connection and the real provider
adapter against a deterministic local HTTP fixture at the profile's fixed
loopback endpoint. It proves that fragmented provider protocols are parsed and
normalized into the expected terminal completion through the gateway framing.

This test is intentionally Linux-only because ADR-0012 relies on Linux
`SO_PEERCRED`. The ordinary adapter unit tests continue to cover malformed,
oversized, redirected, non-loopback, cancelled and incomplete provider
responses.

## Release boundary

The route is compiled only in debug/test builds. The packaged gateway process
does not call it, and release/default startup still exits not-ready before
creating a listener. Only synthetic, developer-authored requests are
constructible; private and ambient input remain unrepresentable on this path.

Remaining Phase 4 work includes the closed production registry and installed
package, adversarial systemd namespace/identity evidence, admission-time
readiness binding, and real-model offline target-Linux evidence for both
providers.
