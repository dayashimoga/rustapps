# PowerShell SBOM Generation Script
Write-Host "Generating CycloneDX Software Bill of Materials (SBOM)..." -ForegroundColor Cyan
$ErrorActionPreference = "Stop"

if (-not (Test-Path "target/sbom")) {
    New-Item -ItemType Directory -Force -Path "target/sbom" | Out-Null
}

cargo cyclonedx --format json --output-pattern "target/sbom/sbom-cyclonedx.json"
if ($LASTEXITCODE -ne 0) {
    Write-Warning "cargo-cyclonedx not installed. Run: cargo install cargo-cyclonedx"
} else {
    Write-Host "SBOM generated at target/sbom/sbom-cyclonedx.json" -ForegroundColor Green
}
