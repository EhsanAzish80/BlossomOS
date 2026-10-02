#!/usr/bin/env bash
set -euo pipefail

proc_root=${BLOSSOM_PROC_ROOT:-/proc}
found=0

for process in "$proc_root"/[0-9]*; do
  [[ -d $process ]] || continue
  pid=${process##*/}
  executable=$(readlink "$process/exe" 2>/dev/null || true)
  [[ -n $executable ]] || continue

  case "$executable" in
    *blossom*" (deleted)"|*/quickshell\ \(deleted\))
      printf 'stale Blossom process: pid=%s exe=%s\n' "$pid" "$executable" >&2
      found=1
      ;;
  esac
done

if ((found != 0)); then
  printf '%s\n' \
    'restart every affected Blossom user service after pacman -U, then retry' >&2
  exit 1
fi

printf '%s\n' 'no stale Blossom executables'
