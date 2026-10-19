#!/usr/bin/env bash
set -euo pipefail

echo "Executing 8-tier test suite across monorepo..."
cargo test --workspace --all-targets --verbose
echo "All tests executed successfully!"
