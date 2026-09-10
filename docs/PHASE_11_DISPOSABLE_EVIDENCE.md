# Phase 11 disposable-media evidence

Status: bounded physical probe completed and restored on the approved disposable
target; physical installation remains unauthorized.

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

## Bounded physical result

On 2026-09-10, after merged launcher commit
`05ffd44411a167ef1a72fe0e247592f24c13c136` and a fresh successful read-only
observation in protected run `34462749602`, the owner executed the reviewed
launcher as root on the frozen host. The terminal result reported
`disposable_test_completed` for the exact approved target digest above. The
probe wrote, read back, restored, and re-read exactly 4,096 bytes at offset
8,388,608. It reported `disposable_probe_completed_and_restored`; therefore the
original bytes were restored before success was returned.

The exact minimized terminal result is preserved as
`distribution/evidence/phase11-disposable-probe-result-20260910.json`. Its
`evidence_source` records that this result came from operator-captured terminal
output. It contains no serial number, filesystem identifier, or disk content.

This completes the successful bounded disposable-media path. Deterministic
tests cover cancellation, ambiguity, unplug or changed-device rejection,
once-only consumption, and failure recovery. Those negative cases do not grant
physical-install authority and were not induced by writing the internal disk.
