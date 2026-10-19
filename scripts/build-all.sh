#!/usr/bin/env bash
set -euo pipefail

echo "Building offline applications monorepo in release mode..."
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --release
echo "Build completed successfully!"
