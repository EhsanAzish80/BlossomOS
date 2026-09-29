# Blossom Desktop Foundation

Status: VM qualification gate open; physical candidate builds remain blocked.

The first physical graphical boot proved that the kernel, compositor, packaged
shell and fixed broker can reach the qualified Intel Mac. It did not prove a
usable desktop or a general-purpose installer. Only a disposable
VM-qualification ISO may now be produced to execute the graphical smoke gate.
Physical candidates and removable-media writes remain blocked.

## Product boundary

Blossom is both a normal personal computer and an agent-native computer. Files,
web browsing, applications, settings and window management must remain useful
without a configured model. Agent surfaces augment that desktop and retain the
existing explicit-approval and local-first security boundaries.

## Required user journeys

### Live system

1. A Blossom-owned UEFI menu offers **Try or install Blossom OS** and a clearly
   labelled troubleshooting entry. The normal path contains no ArchISO branding.
2. A quiet Blossom startup surface replaces routine console chatter. Detailed
   logs remain available through the troubleshooting entry.
3. The complete desktop appears behind a dismissible **Live session** welcome
   card. Closing the card never leaves an empty or unrecognisable workspace.
4. Network setup is optional. Base installation uses only verified files already
   on the image and works offline.
5. The installer opens visibly, reports actionable errors, never silently picks
   a disk and requires an exact destructive confirmation.

### Installed system

1. The installed system contains no live installer action.
2. First-run onboarding is shown once; later boots open the normal desktop.
3. Settings, applications, files and agent configuration persist across restart.
4. A missing model or unavailable network does not prevent ordinary desktop use.

## Desktop surfaces

- A non-focusable wallpaper/background layer sits below application windows and
  never receives an active-window border.
- A reserved top bar contains Blossom identity, workspace/application context,
  network, sound, battery, clock and a system menu.
- A bottom dock contains Applications, Files, Browser, Terminal, Settings and
  Agent, plus indicators for running applications.
- Launcher, network, notifications and agent activity are overlays rather than
  permanent developer controls in the top bar.
- Power and security confirmations are centred, keyboard accessible and remain
  visible above ordinary windows.
- Ordinary applications have discoverable close, minimise, maximise, switching
  and keyboard interactions. Tiling remains an optional advanced workflow.
- Layouts are checked at 1280x720, 1920x1080 and a 2x HiDPI laptop scale.

## Networking and hardware readiness

The network surface distinguishes device unavailable, disabled, disconnected,
connecting, limited and online states. It supports Ethernet and Wi-Fi discovery,
secured network selection, password retry, disconnect and an explicit offline
path. Unsupported hardware names the adapter and offers Ethernet or USB-tethering
guidance instead of blocking the desktop.

Hardware support is released through an explicit matrix. The existing frozen
MacBookPro11,1 installation boundary is not described as generic support. Wider
x86-64 UEFI support requires separate graphics, Wi-Fi, audio, suspend, restart,
shutdown and installation evidence.

## Installer flow

Language and keyboard -> network or offline -> hardware readiness -> disk ->
account -> time zone -> summary -> exact confirmation -> readable progress ->
completion and restart.

The installer retains logs after failure and allows a safe retry. Network is
used only for optional updates, applications and model downloads. Model choice
occurs after the base desktop is usable and is bounded by detected memory,
storage and architecture.

## Pre-build acceptance gate

- [ ] Blossom-owned boot menu and normal startup path contain no Arch branding.
- [ ] Desktop background, top bar and dock use compositor layer surfaces.
- [ ] Closing Welcome reveals the complete desktop.
- [ ] Live-only Install is hidden on installed systems and visibly opens in live mode.
- [ ] Network and installer failure states are rendered and keyboard accessible.
- [ ] Power confirmations are centred.
- [ ] Desktop background cannot gain application focus or an active border.
- [ ] Ordinary application close, minimise, maximise and switching are discoverable.
- [ ] Offline base installation is preserved.
- [ ] Agent absence never blocks the ordinary desktop.
- [ ] Source tests cover live/installed visibility and all supported display scales.
- [ ] Documentation describes only behavior implemented by the candidate.
- [ ] A local graphical smoke run records screenshots and interactions before ISO work.

Only after every item is reviewed may this document be changed to
`Status: physical build gate open` and the deliberate build lock be removed in
the same reviewed change.
