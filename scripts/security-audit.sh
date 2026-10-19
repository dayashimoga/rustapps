#!/usr/bin/env bash
set -euo pipefail

echo "Executing security and license audits..."
cargo audit || true
cargo deny check || true
echo "Security audit scan completed."
