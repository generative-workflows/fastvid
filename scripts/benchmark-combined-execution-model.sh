#!/usr/bin/env bash
# EXP-0107 native high-bit combined predictor/entropy model.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
output="${1:-$repo_dir/artifacts/exp0107-combined-execution.tsv}"
corpus_dir="${2:-$repo_dir/artifacts/corpus-v2}"
control="${3:-$repo_dir/artifacts/exp0104-predictor-band-ladder.tsv}"

bash "$script_dir/benchmark-predictor-bands.sh" "$output" "$corpus_dir" 90
python3 "$script_dir/summarize-combined-execution-model.py" "$output" "$control"
