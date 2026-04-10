#!/usr/bin/env bash

set -euo pipefail

mode="${1:-build}"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

case "$mode" in
  build)
    cmd=(cargo build --workspace --timings)
    ;;
  test)
    cmd=(cargo test --workspace --timings)
    ;;
  *)
    echo "Usage: $0 {build|test}" >&2
    exit 1
    ;;
esac

(
  cd "$repo_root"
  "${cmd[@]}"
)

latest_report="$(find "$repo_root/target/cargo-timings" -maxdepth 1 -name 'cargo-timing-*.html' -type f -print0 | xargs -0 ls -1t | head -n 1)"

echo
echo "Latest cargo timing report:"
echo "  $latest_report"
echo "Stable cargo timing path:"
echo "  $repo_root/target/cargo-timings/cargo-timing.html"
