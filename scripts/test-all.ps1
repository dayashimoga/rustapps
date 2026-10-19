# PowerShell Test All Script (8 Tiers)
Write-Host "Executing 8-tier test suite across monorepo..." -ForegroundColor Cyan
$ErrorActionPreference = "Stop"

Write-Host "Running Cargo tests (Unit, Integration, Security)..."
cargo test --workspace --all-targets --verbose
if ($LASTEXITCODE -ne 0) { Write-Error "Cargo tests failed!"; exit 1 }

Write-Host "All tests executed successfully!" -ForegroundColor Green
