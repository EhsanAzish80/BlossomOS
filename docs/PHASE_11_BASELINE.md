# Phase 11 first physical-device qualification baseline

Status: active. The boundary and read-only classifier are implemented;
physical installation remains blocked pending the separately reviewed safety
increment and real evidence.

Phase 11 freezes one expected `MacBookPro11,1` as the first disposable physical
target. Eligibility is not compatibility, support, or installation proof.

## Ordered checkpoints

1. Accept ADR-0028 and implement the closed, privacy-minimized read-only
   preflight plus a manually dispatched trusted-runner evidence workflow.
   Complete.
2. Run the preflight on the target and confirm its exact model, x86-64 UEFI
   environment, minimum memory, DRM graphics, and internal-storage presence.
   Complete in protected run `34447352126`; see
   `docs/PHASE_11_PHYSICAL_EVIDENCE.md`.
3. Replace automatic physical installation with fail-closed target discovery,
   live-media and mounted-device rejection, power/recovery prerequisites, and
   explicit typed confirmation. Complete: the guard is integrated only with the
   bounded disposable-media probe; physical-install authority remains blocked.
4. Prove destructive behavior only against disposable, independently identified
   media, including cancellation, ambiguity, unplug, and failure recovery.
   Complete for the bounded disposable-media checkpoint. The real approved
   target completed write/read/restore verification, while deterministic tests
   cover cancellation, ambiguity, unplug or changed-device rejection, and
   failure recovery.
5. Install and boot Blossom on the frozen target, then test display, keyboard,
   trackpad, network, audio, battery/power, suspend/resume, recovery, update,
   confirmation, and rollback independently. The once-only physical-install
   harness and exact-target backend are implemented and deterministically
   tested. The separate manual-only physical-candidate source is implemented;
   its ISO must still build and pass review before any internal-disk write can
   be authorized.
6. Publish a Phase 11 exit audit with immutable evidence and explicit hardware
   limitations. Pending.

## Non-goals

This phase does not promise general Intel Mac, laptop, x86-64, daily-driver, or
public-release support. It does not add dual boot, Secure Boot, disk encryption,
or automatic destructive installation.

## Exit rule

Phase 11 remains active until the exact physical target completes the reviewed
installation and lifecycle matrix. A preflight pass alone cannot close it.
