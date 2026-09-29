# Trusted approval manual evidence

Status: pending single-tester manual verification.

This checklist records the part of trusted approval that automation cannot
honestly establish: a person saw the exact Blossom preview and completed or
refused the system password prompt. It is not a usability study and must not be
reported as one.

## Required image boundary

Use the ARM64 trusted-approval image only after its automated gate reports:

```text
BLOSSOM_ARM64_TRUSTED_APPROVAL_READY self_approval=denied missing_agent=denied effects=0 executor_starts=0 bypass=absent
```

Before testing, confirm in the guest that `blossom-qualification` is not
installed and that
`/usr/share/polkit-1/rules.d/49-blossom-model-effect-qualification.rules` does
not exist. Record the image digest and baked source commit below.

- Tester: pending
- Date and time: pending
- Source commit: pending
- Image SHA-256: pending
- VM resources: 4 vCPUs, 8 GiB RAM

## Manual cases

Use the Blossom shell UI, never the qualification driver.

1. Request one workspace-file create. Capture the exact preview before
   continuing. Enter the correct password. Confirm exactly one file is created
   and its path, length and SHA-256 match the preview.
2. Request a new create, enter an incorrect password and confirm zero effects
   plus an authentication-rejected audit entry.
3. Request another create, cancel the password dialog and confirm zero effects
   plus an authentication-rejected audit entry.
4. Complete two separate approvals in succession. Confirm each approval opens
   a new password dialog; a successful first authentication must not authorize
   the second effect.
5. Request another create, leave the password dialog open beyond the preview's
   30-second expiry, then authenticate. Confirm the request is expired and
   produces zero effects.

## Screenshot evidence

Preserve screenshots locally with private data excluded:

- exact Blossom preview: pending
- system PolicyKit password prompt: pending
- second consecutive password prompt: pending
- expired or rejected activity result: pending

Screenshots show what this tester saw. They do not prove the absence of a
lookalike overlay or establish behavior for other users or hardware.

## Result

Pending. Do not claim that model effects require a human password until every
case above has an observed result and the corresponding audit/effect counts are
recorded here.
