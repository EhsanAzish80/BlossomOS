#!/usr/bin/env bash
set -euo pipefail

repo=$(cd "$(dirname "$0")/../.." && pwd -P)
output=${1:-"$repo/.local-phase11-macos"}
profile=${BLOSSOM_COLIMA_PROFILE:-blossom-x86}
context="colima-$profile"
image='archlinux@sha256:694da1fce635e3a14d90751941b08f02c500e8724682f7a00768e7152251ec34'

if [[ $(uname -s) != Darwin ]]; then
  echo "error: this entrypoint is for macOS" >&2
  exit 2
fi
if [[ $(uname -m) != arm64 ]]; then
  echo "error: this entrypoint expects an Apple-silicon Mac" >&2
  exit 2
fi
for command in colima docker shasum; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "error: required command is missing: $command" >&2
    exit 2
  fi
done

if ! colima status "$profile" >/dev/null 2>&1; then
  echo "Starting isolated x86_64 builder VM: $profile"
  colima start "$profile" --arch x86_64 --vm-type qemu --runtime docker \
    --cpus 4 --memory 8 --disk 100 --kubernetes=false
fi

server_arch=$(docker --context "$context" info --format '{{.Architecture}}')
if [[ "$server_arch" != x86_64 && "$server_arch" != amd64 ]]; then
  echo "error: Docker context $context is $server_arch, not x86_64" >&2
  exit 2
fi

mkdir -p "$output"
output=$(cd "$output" && pwd -P)
case "$output" in
  "$repo"/*) ;;
  *) echo "error: output must be inside the BlossomOS repository" >&2; exit 2 ;;
esac
rm -rf "$output/build" "$output/iso"
mkdir -p "$output/iso"

container="blossom-candidate-build-$$"
volume="blossom-candidate-build-$$"
cleanup() {
  docker --context "$context" rm -f "$container" >/dev/null 2>&1 || true
  docker --context "$context" volume rm "$volume" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM
docker --context "$context" volume create "$volume" >/dev/null

docker --context "$context" run --name "$container" --platform linux/amd64 --privileged \
  --volume "$repo:/workspace:ro" \
  --mount "type=volume,source=$volume,target=/candidate" \
  --workdir /workspace \
  "$image" \
  bash -euo pipefail -c '
    cp distribution/evidence/pacman-snapshot.conf /etc/pacman.conf
    pacman -Syu --noconfirm arch-install-scripts archiso base-devel dosfstools \
      gptfdisk python rust zstd
    scripts/distribution/build_physical_candidate.sh /candidate
  '

docker --context "$context" cp "$container:/candidate/iso/." "$output/iso"

(
  cd "$output/iso"
  test "$(find . -maxdepth 1 -name 'blossom-os-*.iso' | wc -l | tr -d ' ')" -eq 1
  shasum -a 256 -c SHA256SUMS
)

echo "Candidate ready: $output/iso"
