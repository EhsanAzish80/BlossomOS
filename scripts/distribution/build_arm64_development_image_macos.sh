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
for command in colima curl docker git python3 qemu-img shasum; do
  command -v "$command" >/dev/null || { echo "error: missing command: $command" >&2; exit 2; }
done

if [[ -n $(git -C "$repo" status --porcelain --untracked-files=no) ]]; then
  echo "error: tracked source changes must be committed before image assembly" >&2
  exit 2
fi
source_commit=$(git -C "$repo" rev-parse HEAD)

registry_value() {
  python3 -c 'import json,sys; value=json.load(open(sys.argv[1]));
for key in sys.argv[2:]: value=value[key]
print(value)' "$@"
}

fetch_exact() {
  local url=$1 expected=$2 destination=$3 actual
  if [[ ! -f $destination ]]; then
    curl -fL --retry 3 "$url" -o "$destination.part"
    mv "$destination.part" "$destination"
  fi
  actual=$(shasum -a 256 "$destination" | awk '{print $1}')
  if [[ $actual != "$expected" ]]; then
    echo "error: pinned input digest mismatch for $destination" >&2
    exit 1
  fi
}

mkdir -p "$output/cache" "$output/image" "$output/work" "$output/diagnostics"
output=$(cd "$output" && pwd -P)
case "$output" in
  "$repo"/*) ;;
  *) echo "error: output must remain inside the repository" >&2; exit 2 ;;
esac
rootfs="$output/cache/$rootfs_name"
fetch_exact "$rootfs_url" "$rootfs_sha256" "$rootfs"

runtime_record="$repo/system/model-runtime/registry/llama-cpp-b10775-aarch64.runtime.json"
model_record="$repo/system/model-runtime/registry/qwen2.5-0.5b-instruct-q4_k_m.model.json"
runtime_archive="$output/cache/$(registry_value "$runtime_record" archive)"
model_file="$output/cache/$(registry_value "$model_record" file)"
model_license="$output/cache/qwen2.5-0.5b-instruct.LICENSE"
fetch_exact "$(registry_value "$runtime_record" url)" \
  "$(registry_value "$runtime_record" sha256)" "$runtime_archive"
fetch_exact "$(registry_value "$model_record" url)" \
  "$(registry_value "$model_record" sha256)" "$model_file"
fetch_exact "$(registry_value "$model_record" license url)" \
  "$(registry_value "$model_record" license sha256)" "$model_license"

if ! colima status "$profile" >/dev/null 2>&1; then
  colima start "$profile" --arch aarch64 --vm-type vz --runtime docker \
    --cpus 6 --memory 10 --disk 30 --kubernetes=false
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
  --env BLOSSOM_IMAGE_COMMIT="$source_commit" \
  --env BLOSSOM_LLAMA_RUNTIME_ARCHIVE="/output/cache/$(basename "$runtime_archive")" \
  --env BLOSSOM_LLAMA_MODEL="/output/cache/$(basename "$model_file")" \
  --env BLOSSOM_LLAMA_MODEL_LICENSE="/output/cache/$(basename "$model_license")" \
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
