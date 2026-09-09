# Blossom OS Vision

## Purpose

Blossom OS is an open-source, agent-native Linux desktop. Its defining idea is
that the desktop exposes a structured, permissioned model of itself to both the
human and a local AI agent.

Blossom is local-first. Files, activity, system context, prompts, tool results,
and memory stay on the device unless the user deliberately enables and scopes an
external service.

## Product principles

1. The agent is a participant in the desktop architecture, not a chat widget.
2. The model and inference provider are replaceable.
3. Structured capabilities are preferred over arbitrary shell commands.
4. No model receives unrestricted root or sudo access.
5. Sensitive actions are previewed, permission-checked, and auditable.
6. Security boundaries are enforced in code outside the model.
7. The shell, agent runtime, broker, executor, and privileged helper are separate
   components with explicit interfaces.
8. Local memory is visible, controllable, exportable, and erasable.
9. Results are verified before Blossom reports success.
10. Components fail closed when policy or identity cannot be established.

## What Blossom is

- An Arch-based Linux desktop built initially around Hyprland and Quickshell.
- A coherent shell and system-services layer with structured desktop context.
- A replaceable local-model runtime with a deterministic tool protocol.
- A capability broker, policy engine, sandboxed executor, and minimal privileged
  helper.
- An open-source system whose security claims are backed by tests and evidence.

## What Blossom is not

- A general-purpose model with `sudo bash`.
- A Linux theme or an assistant installed on an otherwise unrelated desktop.
- A system whose safety depends on a prompt telling the model to behave.
- A cloud service disguised as a local operating system.
- A promise that every Linux action can be safely automated.

## Initial user promise

> Your computer can understand itself, work locally, and show you what it intends
> to do before sensitive changes happen.

## Current reality

The repository began as a desktop prototype and now contains reviewed, bounded
foundations for typed capabilities, exact approval, sandboxed execution,
minimal privilege, provider-neutral local-model protocols, truthful planning,
graphical approval and activity, structured system context, encrypted durable
notes, and an offline installation and rollback lifecycle. Phases 0 through 10
are complete only at their documented boundaries.

Phase 10 public-beta hardening is complete within its reviewed x86-64 UEFI VM
beta-candidate evidence boundary. Blossom OS is still pre-release research
software: there is no supported release, physical-device compatibility claim,
daily-use guarantee, or general-purpose autonomous authority. The
historical prototype remains preserved by the
`prototype-pre-agent-architecture` tag.
