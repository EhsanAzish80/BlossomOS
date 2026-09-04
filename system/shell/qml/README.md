# Phase 6 Quickshell surface

This is the minimal graphical client for the fixed diagnostic slice. It renders
the complete service-authored security preview, offers only `Approve once` and
`Deny`, cancels pending work on Escape, and shows the bounded redacted activity
projection. It does not infer success from a D-Bus reply: the displayed state
and activity originate from the authoritative service projection.

The QML imports the narrow `Blossom.Shell` native plugin. It does not import
`Quickshell.Io`, launch processes, read files, choose D-Bus identifiers, call
Hyprland IPC, handle approval tokens, or construct capabilities and scopes.

The surface is not installed by this checkpoint. Pinned Quickshell/Hyprland
loading, focus behavior, close behavior, accessibility, visual integrity, and
end-to-end execution remain installed-evidence gates.
