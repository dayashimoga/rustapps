# PowerShell Security Audit Script
Write-Host "Executing security and license audits..." -ForegroundColor Cyan
$ErrorActionPreference = "Stop"

Write-Host "Running cargo-audit..."
cargo audit
if ($LASTEXITCODE -ne 0) { Write-Warning "cargo audit reported advisories or is uninstalled." }

Write-Host "Running cargo-deny..."
cargo deny check
if ($LASTEXITCODE -ne 0) { Write-Warning "cargo deny check reported warnings or is uninstalled." }

Write-Host "Security audit scan completed." -ForegroundColor Green
