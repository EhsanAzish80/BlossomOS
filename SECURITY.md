# Security Policy

## Project status

Blossom OS is a pre-alpha research and development project. It is not yet safe to
use as a trusted daily operating system. The preserved prototype lacks the target
permission, sandbox, and privilege boundaries and contains insecure development
defaults.

No release is currently supported with security updates.

## Reporting a vulnerability

Use [GitHub Private Vulnerability Reporting](https://github.com/EhsanAzish80/BlossomOS/security/advisories/new)
for suspected vulnerabilities. The repository setting and private reporting form
were verified enabled on 2026-09-02. Do not post exploitable details in a public
issue.

When reporting, include affected revision, environment, reproduction steps,
impact, and any suggested mitigation. Do not access other people's data or
systems while researching Blossom.

## Security guarantees

Only behavior implemented and verified by tests may be described as a guarantee.
Target properties are defined in `SECURITY_MODEL.md`; they are not claims about
the current prototype.

## Disclosure process

The project targets acknowledgment within three business days and an initial
severity and scope assessment within seven business days. These are response
targets, not guarantees. A critical issue should receive a mitigation or
remediation plan within 30 days when practical; lower-severity work is scheduled
according to impact, exploitability, and release risk.

The maintainer and reporter should coordinate publication after a fix or
mitigation is available. If coordination fails or users face active risk, either
party may disclose responsibly after giving reasonable notice. Reporters may
request public credit, a specific credit name, or anonymity.

No version currently receives security updates. The first supported version and
support lifetime must be stated in its release notes; silence never implies
support. Security fixes must not disclose private report content before the
coordinated disclosure point.
