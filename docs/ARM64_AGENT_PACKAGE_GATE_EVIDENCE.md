# ARM64 local-agent package-gate evidence

Status: complete for the exact package-gate boundary described below on
2026-09-28 at commit `9b9a029`.

This record covers a real Qwen 2.5 0.5B model and the packaged aarch64
llama.cpp runtime installed with `pacman -U` in a disposable ARM64 VM. The
qualification client used the public shell D-Bus interface; the shell broker,
model gateway, provider, resolver, policy, approval store, create-only
publisher, verification, and audit paths were all active.

## Environment

- Architecture: `aarch64`
- Guest resources: 4 vCPUs and 8,109,172 KiB RAM
- Model: Qwen 2.5 0.5B, Q4_K_M
- Commit recorded by every attempt: `9b9a029`
- Profile digest:
  `02602695b5e9c4d299a6d8e1a541df10ec4e8a1195cd88428da8ed79e481873c`
- Installed receipt digest:
  `3a615a8f18d6c9af9f92083fb67c7d603226d41f55a4bed87d634bbf19dc4c2a`

## Result

The real-model warm gate completed 20 consecutive suites on its first streak:
20 total attempts, no failed attempts, no retries, and 100 contiguous passing
case records.

Each suite contained one positive workspace-create case plus direct injection,
invalid indirect injection, valid-but-unwanted indirect injection, and mutated
approval cases.

- All 20 positive cases produced exactly one effect and zero command-executor
  starts.
- All 80 negative cases produced zero effects and zero command-executor starts.
- Every indirect-injection case recorded exactly one bounded context read.
- The valid hostile indirect proposal reached an ordinary exact approval and
  was denied; the invalid variant failed closed.

The following values were identical across all 20 positive cases:

- Parsed-proposal digest:
  `783b76b6ce9a61ae82e09930cad6de3071e6cb8c07539f1a5ba2290433f38123`
- Semantic-preview digest:
  `10e0d093ed385180bf2d9363b7e36c54e9ef60353719bacfe971dfe09148733e`
- Created-file digest:
  `8ecc3eaf0cc045ef429fba1ab2976abce68b3891c116436f72a88462bb869a49`
- Created-file length: 29 bytes

The approval-binding digest was intentionally different for all 20 requests.
It includes per-request identity and expiry and therefore must not be used as a
determinism metric.

## Latency

Observed warm case latency ranges:

| Case | Minimum | Maximum |
| --- | ---: | ---: |
| Positive create | 558 ms | 624 ms |
| Direct injection | 7,208 ms | 7,358 ms |
| Invalid indirect injection | 781 ms | 832 ms |
| Valid indirect injection | 738 ms | 768 ms |
| Mutated approval | 524 ms | 553 ms |

The overall maximum, 7,358 ms, was the direct-injection case, not either
two-turn indirect-injection case.

## Preserved raw evidence

The two append-only JSONL records are retained locally under
`.local-arm64/package-gate/` and are intentionally not committed because local
qualification artifacts are excluded from the source tree.

- `real-two-turn-9b9a029-warm20.jsonl`:
  SHA-256 `f08d1e40cbe95e59e9a20ad1292eac9b0ba13a0dda2cb5b367dcd5bec6dd664b`
- `real-two-turn-9b9a029-warm20-suites.jsonl`:
  SHA-256 `0cd14cfa5ed61a14ac4e6c7ca2fc9a5409307f8d55885d81b416f6e8fad22b87`

## Limitations

- Approval was submitted by the qualification client, not an independently
  authenticated human interaction.
- This used a 0.5B model in an ARM64 VM.
- The implementation and evidence have not received independent review.
- This does not cover x86-64 or the physical Intel MacBook.
- This is an installed-package gate, not an ARM64 image gate.
- It records deterministic parsed proposals and semantic previews, not byte-for-
  byte equality of the SSE transport. SSE request and timestamp metadata vary
  per request and are outside the trusted proposal semantics.

