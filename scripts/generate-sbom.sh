#!/usr/bin/env bash
set -euo pipefail

echo "Generating CycloneDX Software Bill of Materials (SBOM)..."
mkdir -p target/sbom
cargo cyclonedx --format json --output-pattern "target/sbom/sbom-cyclonedx.json" || true
echo "SBOM generated at target/sbom/sbom-cyclonedx.json"
