#!/usr/bin/env bash
set -euo pipefail

echo "Generating code coverage report (target >= 90%)..."
cargo llvm-cov --workspace --all-targets --lcov --output-path target/coverage/lcov.info --html --output-dir target/coverage/html
echo "Coverage report generated at target/coverage/html/index.html"
