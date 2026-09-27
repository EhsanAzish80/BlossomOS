#!/usr/bin/env bash
set -euo pipefail

repo=$(cd "$(dirname "$0")/../.." && pwd -P)
output=${1:-"$repo/.local-arm64"}
profile=${BLOSSOM_ARM64_COLIMA_PROFILE:-blossom-arm64}
context="colima-$profile"
rootfs_name=ArchLinuxARM-aarch64-latest.tar.gz
rootfs_url=${BLOSSOM_ARM64_ROOTFS_URL:-https://ca.us.mirror.archlinuxarm.org/os/$rootfs_name}
rootfs_sha256=${BLOSSOM_ARM64_ROOTFS_SHA256:-42a4eeaa038994ffd31fa173256ef2f0ef511358eeb41b9ea1f8626391b9b319}

if [[ $(uname -s) != Darwin || $(uname -m) != arm64 ]]; then
  echo "error: this builder requires an Apple-silicon Mac" >&2
  exit 2
fi
for command in colima curl docker qemu-img shasum; do
  command -v "$command" >/dev/null || { echo "error: missing command: $command" >&2; exit 2; }
done

mkdir -p "$output/cache" "$output/image" "$output/work" "$output/diagnostics"
output=$(cd "$output" && pwd -P)
case "$output" in
  "$repo"/*) ;;
  *) echo "error: output must remain inside the repository" >&2; exit 2 ;;
esac
rootfs="$output/cache/$rootfs_name"
if [[ ! -f "$rootfs" ]]; then
  curl -fL --retry 3 "$rootfs_url" -o "$rootfs.part"
  mv "$rootfs.part" "$rootfs"
fi
actual=$(shasum -a 256 "$rootfs" | awk '{print $1}')
if [[ "$actual" != "$rootfs_sha256" ]]; then
  echo "error: ARM64 rootfs digest mismatch: expected $rootfs_sha256, got $actual" >&2
  exit 1
fi

if ! colima status "$profile" >/dev/null 2>&1; then
  colima start "$profile" --arch aarch64 --vm-type vz --runtime docker \
    --cpus 6 --memory 10 --disk 60 --kubernetes=false
fi
server_arch=$(docker --context "$context" info --format '{{.Architecture}}')
if [[ "$server_arch" != aarch64 && "$server_arch" != arm64 ]]; then
  echo "error: Docker context $context is $server_arch, not aarch64" >&2
  exit 2
fi

raw="$output/image/blossom-os-arm64-development.raw"
qcow="$output/image/blossom-os-arm64-development.qcow2"
rm -f "$raw" "$qcow"
docker --context "$context" run --rm --platform linux/arm64 --privileged \
  --dns 1.1.1.1 \
  --volume "$repo:/workspace:ro" \
  --volume "$output:/output" \
  --workdir /workspace \
  ubuntu:24.04 bash -euo pipefail -c '
    export DEBIAN_FRONTEND=noninteractive
    apt-get update
    apt-get install -y --no-install-recommends ca-certificates curl dosfstools e2fsprogs gdisk libarchive-tools mount rsync util-linux
    scripts/distribution/build_arm64_development_image.sh \
      /output/cache/ArchLinuxARM-aarch64-latest.tar.gz \
      /output/image/blossom-os-arm64-development.raw \
      /output/work/arm64-image
  ' 2>&1 | tee "$output/diagnostics/build-arm64.log"

qemu-img convert -f raw -O qcow2 -c "$raw" "$qcow"
qemu-img check "$qcow"
shasum -a 256 "$qcow" >"$output/image/SHA256SUMS"
rm -f "$raw"
echo "ARM64 development image ready: $qcow"
