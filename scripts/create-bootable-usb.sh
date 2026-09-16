#!/bin/sh
set -eu

repository=$(cd -- "$(dirname -- "$0")/.." && pwd)
exec python3 "$repository/scripts/distribution/media_creator.py" "$@"
