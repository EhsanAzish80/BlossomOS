# Exact-effect installed-package evidence

Status: complete for the ARM64 installed-package boundary described below on
2026-10-01. The source tree was based on commit
`34d5dd1e0975f897a607e2ad11d614d80dd97ba0`; the commit containing this record
and its launcher/accessibility regression fixes is the authoritative final
source state.

This record closes the exact-effect disclosure finding documented in
`TRUSTED_APPROVAL_MANUAL_EVIDENCE.md`. It is a local package gate, not a clean
image gate, and did not run a GitHub-hosted build.

## Environment and installation boundary

- Architecture: `aarch64`
- Guest resources: 4 vCPUs, 8 GiB RAM
- Base: credential-free ARM64 development qcow2
- Test disk: disposable qcow2 overlay, deleted after evidence capture
- Package path: locally built in the ARM64 VM and installed with `pacman -U`
- Installed packages: `blossom-core 0.9.0-3`, `blossom-shell 0.9.0-2`, and
  `blossom-trusted-approval-qualification 0.1.0-2`
- Authentication: random one-run password passed through a file descriptor to
  `pkttyagent`, then locked and cleared by the qualification orchestrator

The interactive ARM64 launcher now always supplies
`org.qemu.guest_agent.0`. The previously inactive guest-agent observation was
reproduced only when the host launch omitted that virtio port. With the port
present, `qemu-guest-agent.service` started and handled guest commands.

## Real PAM cases

Eight append-only cases passed:

| Case | Prompts | Effects | Executor starts | Result |
| --- | ---: | ---: | ---: | --- |
| Correct password | 1 | 1 | 0 | authorized and verified |
| Wrong password | 1 | 0 | 0 | rejected |
| Cancel | 1 | 0 | 0 | rejected |
| Expiry | 1 | 0 | 0 | challenge expired |
| Repeated approvals | 2 | 2 | 0 | two independent authorizations |
| Rate limit | 3 for 4 requests | 0 | 0 | fourth request rate limited |
| Cooldown reset | 1 | 0 | 0 | prompting resumed after cooldown |
| Missing agent | 0 | 0 | 0 | failed closed |

The preserved local JSONL digest was
`ed2c4fba635deea26c08a3f8bd4206ee6911d0ead0af5083b92704301f5b7b82`.
It remains a local generated artifact rather than tracked source.

## Original failing request

The literal request from the manual finding was rerun through an installed QML
client, the production shell D-Bus boundary, the deterministic parser, the
installed approval panel, real PolicyKit/PAM authentication, create-only
publication, verification, and audit:

```text
create manual-proof.txt containing clean image password approval works
```

Observed result:

- proposal source: `parsed_directly`
- displayed original request: exact text above
- displayed destination: `/home/blossom/Workspace/manual-proof.txt`
- displayed content: `clean image password approval works`
- displayed source label: `Parsed directly from your request`
- password prompts: exactly one
- terminal status: `verified`
- file length: 35 UTF-8 bytes
- file SHA-256:
  `ea1fccd3515f1b929ba678c49906afe6d4eb624e62fc9ba6e91c37ceb18c7cb7`
- on-disk bytes: exact match for the displayed and requested content

AT-SPI read the displayed request, destination, complete content, and proposal
source from the live approval window before invoking its accessible
`Approve once` action. This rerun also found that the installed desktop and
approval services did not force Qt accessibility on, while the older fixture
launcher did. Both packaged user units now set
`QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1`, with source checks preventing regression.

## Parser and model-route contracts

- The real-model qualification driver requires `model_proposed` for the
  positive, mutation, and injection/model cases, so the 20-run model gate
  cannot silently take the deterministic route.
- A request that matches the deterministic syntax but contains an invalid name
  returns `InvalidDirectRequest`, leaves the workspace unchanged, and never
  invokes the model. This was run on Linux ARM64.
- Parsing splits at the first `containing` delimiter. The focused test preserves
  later occurrences inside the content.

## Automated checks

- `cargo test --workspace --all-targets --locked`: passed
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed
- `cargo fmt --all -- --check`: passed
- shell packaging and QML source checks: passed
- ARM64 launcher, qualification-driver, and physical-candidate Python tests:
  13 passed
- Linux ARM64 direct-parser tests: 3 passed
- Linux ARM64 matched-invalid fail-closed test: 1 passed
- maximum-size installed approval-component AT-SPI check: passed

## Limitations

- This is a local ARM64 VM installed-package result, not physical Intel
  MacBook evidence.
- The original-request run used an automated `pkttyagent` responder with a
  one-run password. It proves real PAM authentication and exact UI disclosure,
  not human usability or resistance to a convincing phishing overlay.
- No dedicated clean image was built for these small package and launcher
  changes. They are intentionally scheduled to ride the next feature's clean
  image gate.
