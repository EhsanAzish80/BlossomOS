# Blossom package fast loop

Use this loop for service and package changes in a disposable VM overlay. A
clean image build remains a separate qualification gate.

1. Build the changed packages locally in the ARM64 VM.
2. Install them with `pacman -U`.
3. Reload system and user-manager unit definitions.
4. Restart every user or system service supplied by the changed packages.
5. Run `scripts/verify_no_stale_blossom_processes.sh`. A process whose
   `/proc/<pid>/exe` ends in `(deleted)` is still executing a replaced binary;
   the loop fails until the affected service is restarted.
6. Run the relevant probe and print the journal for any failed unit.

Do not treat this loop as clean-image, boot, installer or physical-hardware
evidence. A/B updates reboot into the new deployment, so the stale-process
tripwire is specifically protection for development package iteration.
