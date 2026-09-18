#!/usr/bin/env bash
set -euo pipefail

repo=$(cd "$(dirname "$0")/../.." && pwd -P)
output=${1:-"$repo/.local-phase11-macos"}
mode=${BLOSSOM_CANDIDATE_MODE:-physical}
profile=${BLOSSOM_COLIMA_PROFILE:-blossom-x86}
context="colima-$profile"
image='archlinux@sha256:694da1fce635e3a14d90751941b08f02c500e8724682f7a00768e7152251ec34'

case "$mode" in
  physical|vm-qualification) ;;
  *) echo "error: BLOSSOM_CANDIDATE_MODE must be physical or vm-qualification" >&2; exit 2 ;;
esac

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
mkdir -p "$output/iso" "$output/diagnostics"

container="blossom-candidate-build-$$"
volume="blossom-candidate-build-$$"
build_log="$output/diagnostics/build.log"
state_file="$output/diagnostics/build-state.env"
build_succeeded=false
cleanup() {
  local status=$?
  if [[ "$build_succeeded" == true ]]; then
    docker --context "$context" rm -f "$container" >/dev/null 2>&1 || true
    docker --context "$context" volume rm "$volume" >/dev/null 2>&1 || true
    rm -f "$state_file"
  else
    {
      printf 'BLOSSOM_BUILD_STATUS=failed_or_interrupted\n'
      printf 'BLOSSOM_BUILD_EXIT=%q\n' "$status"
      printf 'BLOSSOM_BUILD_CONTEXT=%q\n' "$context"
      printf 'BLOSSOM_BUILD_CONTAINER=%q\n' "$container"
      printf 'BLOSSOM_BUILD_VOLUME=%q\n' "$volume"
      printf 'BLOSSOM_BUILD_LOG=%q\n' "$build_log"
    } >"$state_file"
    echo "Build did not complete; preserving diagnostics and Docker state:" >&2
    echo "  log: $build_log" >&2
    echo "  state: $state_file" >&2
    echo "  container: $container" >&2
    echo "  volume: $volume" >&2
  fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
docker --context "$context" volume create "$volume" >/dev/null

docker --context "$context" run --detach --name "$container" --platform linux/amd64 --privileged \
  --dns 1.1.1.1 \
  --env "BLOSSOM_CANDIDATE_MODE=$mode" \
  --volume "$repo:/workspace:ro" \
  --mount "type=volume,source=$volume,target=/candidate" \
  --workdir /workspace \
  "$image" \
  bash -euo pipefail -c '
    cp distribution/evidence/pacman-snapshot.conf /etc/pacman.conf
    pacman -Syu --noconfirm arch-install-scripts archiso base-devel cmake dosfstools \
      gptfdisk ninja python qt6-base qt6-declarative rust zstd
    scripts/distribution/build_physical_candidate.sh /candidate
  ' >/dev/null

# Follow a detached build so an interrupted terminal cannot silently erase the
# container, volume, and only useful failure evidence. The EXIT trap preserves
# all three unless the candidate is copied and verified successfully.
set +e
docker --context "$context" logs --follow "$container" 2>&1 | tee "$build_log"
logs_status=${PIPESTATUS[0]}
container_status=$(docker --context "$context" wait "$container" 2>/dev/null)
wait_status=$?
set -e
if [[ $logs_status -ne 0 || $wait_status -ne 0 || ! "$container_status" =~ ^[0-9]+$ || $container_status -ne 0 ]]; then
  echo "error: candidate build container failed (logs=$logs_status wait=$wait_status container=${container_status:-unknown})" >&2
  exit 1
fi

docker --context "$context" cp "$container:/candidate/iso/." "$output/iso"

(
  cd "$output/iso"
  test "$(find . -maxdepth 1 -name 'blossom-os-*.iso' | wc -l | tr -d ' ')" -eq 1
  shasum -a 256 -c SHA256SUMS
)

build_succeeded=true
echo "Candidate ready ($mode): $output/iso"
