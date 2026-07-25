#!/usr/bin/env bash
# EXP-0088 allocation-reusing portable fixed-block kernel benchmark.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
output="${1:-$repo_dir/artifacts/exp0088-block-pack-kernel.tsv}"
iterations="${2:-100000}"

cargo build --release --bin block_pack_bench --manifest-path "$repo_dir/Cargo.toml"
"$repo_dir/target/release/block_pack_bench" "$iterations" > "$output"
echo "results: $output"
