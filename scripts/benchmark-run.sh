#!/usr/bin/env bash
set -euo pipefail

echo "Executing Criterion performance benchmarks..."
cargo bench --workspace
echo "Benchmark reports available in target/criterion/report/index.html"
