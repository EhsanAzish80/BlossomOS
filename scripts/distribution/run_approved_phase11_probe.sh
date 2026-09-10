#!/usr/bin/env bash
set -euo pipefail

# This helper is intentionally frozen to the single Phase 11 disposable-media
# authorization already recorded for /dev/sdc. The Python guard still performs
# a fresh observation comparison before the bounded probe can open the device.
repo_root=${BLOSSOM_PHASE11_REPO_ROOT:?missing reviewed repository path}
cd "$repo_root"

current=$(mktemp /tmp/blossom-phase11-current.XXXXXX.json)
trap 'rm -f "$current"' EXIT

python3 scripts/distribution/physical_device_observer.py \
  --purpose disposable_test \
  --host-preflight-result eligible_for_qualification \
  --ac-power ready \
  --recovery-media ready \
  --challenge 8a25f735e9890322c41976254aff546e \
  > "$current"

python3 -m scripts.distribution.run_disposable_media_test \
  --state /var/lib/blossom/phase11-claims/f7436d816d5efe2c8d7e8da02b31755a10e0124edfbfde09f0b8f3e96ad6f911.claim \
  --initial distribution/evidence/phase11-device-observation-34460218314.json \
  --current "$current" \
  --confirmation 'ERASE /dev/sdc f7436d816d5efe2c'
