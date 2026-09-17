# Blossom OS hardware support matrix

Status: qualification matrix; absence from this table means unverified, not
unsupported.

| Target | Boot | Graphics | Input | Network | Audio | Install | Suspend | Power | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Generic x86-64 UEFI VM | Proven | Proven | Proven | Virtual adapter only | Not qualified | VM boundary proven | Not qualified | Reboot proven | Qualification only |
| MacBookPro11,1 | Proven live | Intel display proven | Keyboard, touchpad and pointer observed | Adapter not yet identified; Ethernet or tethering fallback required | Not yet qualified | Frozen internal-target flow only | Not yet qualified | Restart/shutdown incomplete | Physical qualification active |
| Other x86-64 UEFI hardware | Unverified | Unverified | Unverified | Unverified | Unverified | Not authorised | Unverified | Unverified | No support claim |

## Network release rules

- The base system installs from the ISO without network access.
- NetworkManager owns wired and wireless setup; the normal desktop opens a
  graphical connection editor rather than a terminal-only tool.
- The exact PCI/USB identity and driver of every physical Wi-Fi adapter must be
  recorded before support is claimed.
- Firmware presence is not proof that association, authentication, DHCP, DNS or
  reconnect works.
- Unsupported adapters produce a visible explanation and Ethernet or USB
  tethering guidance. They do not prevent live desktop use or offline install.
- Agent/model availability is reported separately from desktop connectivity.

## Evidence required to change a cell to proven

Record the candidate digest, machine identity, device identity and driver, then
exercise cold boot, reconnect, restart and failure recovery. Network evidence
must cover secured Wi-Fi and Ethernet where the hardware exposes them. Audio,
suspend and power evidence are independent gates.
