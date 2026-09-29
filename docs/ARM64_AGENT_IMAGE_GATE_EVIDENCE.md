# ARM64 local-agent image-gate evidence

Status: complete for the exact baked-image boundary described below on
2026-09-29 at commit `a30d628`.

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
- Source commit baked into the image: `a30d628`
- Qualification duration: 214 seconds

## Artifact verification

The build completed its qcow2 integrity check, the checked-in builder's raw
intermediate was removed, and the build work directory returned to zero bytes.
The host retained 113 GiB free after the build.

- Image SHA-256:
  `07aca9bba4d801af03ca1ae2d0d41d9cfe4a7939e9ba892fc16d670800207707`
- Image size: 4,267,638,784 bytes
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
  SHA-256 `a46778bad24dff7df486f3e3c177c78d945e7180cf8a8ff8eec206df66fac1eb`
- `.local-arm64/qualification/serial.log`:
  SHA-256 `1af9474895f83d6aaaeabe6cd5afb4690336f0c6710517c4a3e9bd23e71e7d1d`

The result log contains both terminal markers:

```text
BLOSSOM_ARM64_AGENT_GATE_READY attempts=20 records=100 effects=20 executor_starts=0
BLOSSOM_ARM64_QUALIFICATION_READY desktop=active display=1280x800@1 network=https installer=active browser=profile seconds=214
```

## Limitations

- Approval was submitted by the qualification client, not an independently
  authenticated human interaction.
- This used a 0.5B model in an ARM64 VM.
- The implementation and evidence have not received independent review.
- This does not cover x86-64 or the physical Intel MacBook.
- The image gate validates boot, desktop readiness, networking, installer
  availability, browser-profile provisioning, and the bounded agent loop. It
  does not establish physical-device compatibility or dual-root A/B update and
  rollback behavior.
