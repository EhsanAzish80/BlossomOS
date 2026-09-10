# Phase 11 first physical-device qualification baseline

Status: active. The boundary and read-only classifier are implemented;
physical installation remains blocked pending the separately reviewed safety
increment and real evidence.

Phase 11 freezes one expected `MacBookPro11,1` as the first disposable physical
target. Eligibility is not compatibility, support, or installation proof.

## Ordered checkpoints

1. Accept ADR-0028 and implement the closed, privacy-minimized read-only
   preflight. Complete.
2. Run the preflight on the target and confirm its exact model, x86-64 UEFI
   environment, minimum memory, DRM graphics, and internal-storage presence.
   Pending physical evidence.
3. Replace automatic physical installation with fail-closed target discovery,
   live-media and mounted-device rejection, power/recovery prerequisites, and
   explicit typed confirmation. Pending separate review.
4. Prove destructive behavior only against disposable, independently identified
   media, including cancellation, ambiguity, unplug, and failure recovery.
   Pending.
5. Install and boot Blossom on the frozen target, then test display, keyboard,
   trackpad, network, audio, battery/power, suspend/resume, recovery, update,
   confirmation, and rollback independently. Pending.
6. Publish a Phase 11 exit audit with immutable evidence and explicit hardware
   limitations. Pending.

## Non-goals

This phase does not promise general Intel Mac, laptop, x86-64, daily-driver, or
public-release support. It does not add dual boot, Secure Boot, disk encryption,
or automatic destructive installation.

## Exit rule

Phase 11 remains active until the exact physical target completes the reviewed
installation and lifecycle matrix. A preflight pass alone cannot close it.
