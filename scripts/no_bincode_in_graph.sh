#!/usr/bin/env bash
# bincode (RUSTSEC-2025-0141, unmaintained) must not enter alice-serial's own
# dependency graph: the differential check against it lives only in fuzz/,
# which has its own lockfile. Lists every package of the graph with all
# features, all targets and every edge kind, and fails when bincode is one of
# them or is in Cargo.lock; a listing of 0 packages compared nothing and fails.
set -euo pipefail
cd "$(dirname "$0")/.."

pkgs=$(cargo tree --locked --all-features --target all -e normal,build,dev \
  --prefix none --format '{p}' | awk '{print $1}' | sort -u)
n=$(printf '%s\n' "$pkgs" | grep -c . || true)
if [ "$n" -eq 0 ]; then
  echo "cargo tree listed no packages: the check compared nothing" >&2
  exit 1
fi
if printf '%s\n' "$pkgs" | grep -qx 'bincode'; then
  echo "bincode is in alice-serial's dependency graph:" >&2
  cargo tree --locked --all-features --target all -e normal,build,dev -i bincode >&2
  exit 1
fi
if grep -qx 'name = "bincode"' Cargo.lock; then
  echo "bincode is in Cargo.lock" >&2
  exit 1
fi
echo "bincode absent: checked $n packages and Cargo.lock"
