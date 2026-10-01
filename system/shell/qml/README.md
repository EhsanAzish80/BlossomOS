# Phase 6 shell surface

This is the minimal graphical client for the fixed diagnostic and local-agent slices. It renders
the complete service-authored security preview, offers only `Approve once` and
`Deny`, cancels pending work on Escape, and shows the bounded redacted activity
projection. It does not infer success from a D-Bus reply: the displayed state
and activity originate from the authoritative service projection.

Phase 7 adds only the service-verified, expiring battery status, percentage,
closed battery state, and closed network connectivity enum. QML cannot select a provider, source, lifetime, polling
interval, D-Bus target, or additional field.

The security QML runs in the dedicated standard Qt `blossom-shell-ui` process
so its complete control tree is available through Linux AT-SPI. The pinned
Quickshell process owns the unprivileged wallpaper, top bar, dock, launcher,
welcome and activity surfaces. It does not host the approval ceremony because
its proxy hierarchy does not publish child controls to AT-SPI. Closing Welcome
reveals the complete layer-shell desktop rather than closing the shell.

The QML imports the narrow `Blossom.Shell` native plugin. It does not import
`Quickshell.Io`, launch processes, read files, choose D-Bus identifiers, call
Hyprland IPC, handle approval tokens, or construct capabilities and scopes.
The Agent panel accepts one bounded prompt and sends it through the fixed shell
D-Bus method. Model proposals still resolve, enter policy, and require the
separate exact-preview approval ceremony before any effect.

Ordinary shell presentation uses the local `Petal` QML singleton for color,
type, spacing, radius, motion, and focus tokens. Petal contains presentation
data only and cannot import the broker plugin or define executable behavior.
IBM Plex is supplied by the packaged `ttf-ibm-plex` dependency, with its OFL
licence installed alongside the shell package.

The approval window deliberately does not import Petal. It retains a fixed
security palette and the header `Blossom approval · drawn by the system broker`
so ordinary theme changes cannot silently make authorization look like another
command-bar result. This is a recognizable cue, not protection against a
same-user process drawing a lookalike window; ADR-0034 records that remaining
overlay-phishing risk.

The approval overlay focuses denial as its safe default, cycles Tab and Backtab
only between the two decision controls, maps assistive press actions to the same
fixed broker calls, and maps the standard Qt window closing signal to pending
approval cancellation. Status is exposed as an accessibility alert. These
semantics do not grant keyboard or assistive technology any additional
authority.

Pinned Quickshell/Hyprland loading has passing installed evidence. Physical
input, screen-reader behavior, visual integrity, and end-to-end interaction
remain evidence gates until exercised on the installed surface.
