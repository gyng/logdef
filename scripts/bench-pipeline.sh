#!/usr/bin/env bash

set -euo pipefail

repeats="${1:-1}"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_dir="$repo_root/target/benchmarks"
timestamp="$(date -u +"%Y%m%dT%H%M%SZ")"
csv_path="$out_dir/pipeline-$timestamp.csv"
latest_path="$out_dir/pipeline-latest.csv"

mkdir -p "$out_dir"

run_case() {
  local label="$1"
  local cmd="$2"

  for run in $(seq 1 "$repeats"); do
    echo "==> $label (run $run/$repeats)"
    local start_ns
    local end_ns
    local elapsed_ms

    start_ns="$(date +%s%N)"
    (
      cd "$repo_root"
      bash -lc "$cmd"
    )
    end_ns="$(date +%s%N)"
    elapsed_ms=$(((end_ns - start_ns) / 1000000))

    printf '%s,%s,%s\n' "$label" "$run" "$elapsed_ms" >> "$csv_path"
  done
}

echo "label,run,elapsed_ms" > "$csv_path"

run_case "cargo-test" "cargo test"
run_case "make-check" "make check"
run_case "make-build" "make build"
run_case "bench-core" "cargo run --release -p understory-core --example engine_bench --quiet"
run_case "bench-scenario" "cargo run --release -p understory-core --example encounter_scenario_bench --quiet"

cp "$csv_path" "$latest_path"

echo
echo "Wrote benchmark results to:"
echo "  $csv_path"
echo "  $latest_path"
echo
awk -F, '
NR == 1 { next }
{
  count[$1] += 1
  total[$1] += $3
  if (!(min[$1]) || $3 < min[$1]) min[$1] = $3
  if ($3 > max[$1]) max[$1] = $3
}
END {
  printf "%-16s %8s %10s %10s %10s\n", "label", "runs", "avg_ms", "min_ms", "max_ms"
  for (label in count) {
    printf "%-16s %8d %10.1f %10d %10d\n", label, count[label], total[label] / count[label], min[label], max[label]
  }
}
' "$csv_path"
