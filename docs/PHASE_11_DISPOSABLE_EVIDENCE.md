# Phase 11 disposable-media evidence

Status: pre-write observation accepted; bounded physical probe pending.

Protected workflow run `34460218314` completed on 2026-09-10 from reviewed
`main`. It reconfirmed the frozen `MacBookPro11,1`, observed the exact bounded
disk inventory, selected one unmounted USB disposable-test target, and uploaded
the minimized evidence without writing any device.

The artifact is preserved byte-for-byte under `distribution/evidence/`:

| File | SHA-256 |
| --- | --- |
| `phase11-device-preflight-34460218314.json` | `5de11cb6ab30605ef4bcda084d2d73994a94269f3e99e9ac436f19b9b1c32c3c` |
| `phase11-device-observation-34460218314.json` | `805e4cabbd4f70572c103038c1e6292bd8f98694738c2ff6ac7ce527bc8c1394` |
| `phase11-device-decision-34460218314.json` | `13928b91fd0e6fbffd8cb8c58f65432a83fc179b6a0d4e96fb78571af6ab234b` |

The live installed disk is `/dev/sda`; the mounted recovery medium is
`/dev/sdd`; and the sole unmounted disposable target is the 500,107,862,016-byte
Toshiba USB disk at `/dev/sdc`. The accepted operator confirmation is bound to
target digest
`f7436d816d5efe2c8d7e8da02b31755a10e0124edfbfde09f0b8f3e96ad6f911`.

These records contain no serial number, filesystem UUID, MAC address, SSID,
username, file content, or content-derived fingerprint. They authorize only the
separately reviewed bounded disposable probe. They do not authorize physical
installation or any write to the internal disk.
