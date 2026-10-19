# PowerShell Benchmark Execution Script
Write-Host "Executing Criterion performance benchmarks..." -ForegroundColor Cyan
$ErrorActionPreference = "Stop"

cargo bench --workspace
if ($LASTEXITCODE -ne 0) { Write-Error "Benchmarks failed!"; exit 1 }

Write-Host "Benchmark reports available in target/criterion/report/index.html" -ForegroundColor Green
