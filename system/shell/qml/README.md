# Phase 6 Quickshell surface

This is the minimal graphical client for the fixed diagnostic slice. It renders
the complete service-authored security preview, offers only `Approve once` and
`Deny`, cancels pending work on Escape, and shows the bounded redacted activity
projection. It does not infer success from a D-Bus reply: the displayed state
and activity originate from the authoritative service projection.

The QML imports the narrow `Blossom.Shell` native plugin. It does not import
`Quickshell.Io`, launch processes, read files, choose D-Bus identifiers, call
Hyprland IPC, handle approval tokens, or construct capabilities and scopes.

The approval overlay focuses denial as its safe default, cycles Tab and Backtab
only between the two decision controls, maps assistive press actions to the same
fixed broker calls, and maps the pinned Quickshell `closed` signal to pending
approval cancellation. Status is exposed as an accessibility alert. These
semantics do not grant keyboard or assistive technology any additional
authority.

Pinned Quickshell/Hyprland loading has passing installed evidence. Physical
input, screen-reader behavior, visual integrity, and end-to-end interaction
remain evidence gates until exercised on the installed surface.
