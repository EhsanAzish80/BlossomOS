# ARM64 local-agent image-gate evidence

Status: complete for the exact baked-image boundary described below on
2026-09-29 at commit `3bae68c`.

This record covers a freshly built Blossom OS ARM64 development qcow2 booted
under QEMU with Apple Hypervisor acceleration. The image ran the same public
shell D-Bus qualification path used by the installed-package gate, with the
packaged Qwen 2.5 0.5B model, llama.cpp provider, gateway, shell broker,
resolver, policy, approval store, create-only publisher, verification, and
audit path all active inside the guest.

## Environment

- Host: Apple Silicon macOS
- Guest architecture: `aarch64`
- Guest resources: 4 vCPUs and 8 GiB RAM
- Disk: 20 GiB virtual qcow2, 4.0 GiB allocated
- Display: VirtIO GPU at 1280 x 800, scale 1
- Network: QEMU user networking with a successful HTTPS probe
- Model: Qwen 2.5 0.5B, Q4_K_M
- Source commit baked into the image: `3bae68c`
- Qualification duration: 206 seconds

## Artifact verification

The build completed its qcow2 integrity check, the checked-in builder's raw
intermediate was removed, and the build work directory returned to zero bytes.
The host retained 44 GiB free after the build.

- Image SHA-256:
  `e9c475185f8d301d90ef24301b2846712dc2b5a1dadfaabbbf10db0057118eef`
- Image size: 4,281,663,488 bytes
- `qemu-img check`: no errors

## Agent result

The image gate completed 20 consecutive real-model suites:

- Attempts: 20
- Case records: 100
- Intended workspace-create effects: exactly 20
- Command-executor starts: zero
- Negative-case effects: zero

Each suite contained one approved workspace-create case and four adversarial
cases: direct injection, invalid indirect injection, valid-but-unwanted
indirect injection, and approval mutation. The intended create occurred once
per suite. The adversarial cases caused neither a file effect nor a command
execution.

The following deterministic values matched the earlier package gate:

- Parsed-proposal digest:
  `783b76b6ce9a61ae82e09930cad6de3071e6cb8c07539f1a5ba2290433f38123`
- Semantic-preview digest:
  `10e0d093ed385180bf2d9363b7e36c54e9ef60353719bacfe971dfe09148733e`
- Created-file digest:
  `8ecc3eaf0cc045ef429fba1ab2976abce68b3891c116436f72a88462bb869a49`

## Desktop result

The qualification probe also observed:

- graphical desktop active
- display at 1280 x 800, scale 1
- HTTPS networking working
- graphical installer active
- browser profile available

These checks establish that the agent gate ran from the baked graphical image,
not from a separately prepared package-test guest.

## Preserved raw evidence

The raw evidence is retained locally and intentionally excluded from Git:

- `.local-arm64/qualification/result.log`:
  SHA-256 `9da5a54ce0590b9554ff6156abb9e0ebfe4c423e76634bfda7b6a11b70efba57`
- `.local-arm64/qualification/serial.log`:
  SHA-256 `eb0729d88268dc2e531694c93dcf0fd15fe2c0d20c843c9da6f9c6d187b5122a`

The result log contains both terminal markers:

```text
BLOSSOM_ARM64_AGENT_GATE_READY attempts=20 records=100 effects=20 executor_starts=0
BLOSSOM_ARM64_QUALIFICATION_READY desktop=active display=1280x800@1 network=https installer=active browser=profile seconds=206
```

## Limitations

- Approval was submitted by the qualification client through the
  qualification-only PolicyKit rule. Password authentication was bypassed, so
  this proves exact pipeline wiring but not independently authenticated human
  intent or the production password prompt.
- This used a 0.5B model in an ARM64 VM.
- The implementation and evidence have not received independent review.
- This does not cover x86-64 or the physical Intel MacBook.
- The image gate validates boot, desktop readiness, networking, installer
  availability, browser-profile provisioning, and the bounded agent loop. It
  does not establish physical-device compatibility or dual-root A/B update and
  rollback behavior.
