# PowerShell Build All Script
Write-Host "Building offline applications monorepo in release mode..." -ForegroundColor Cyan
$ErrorActionPreference = "Stop"

cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { Write-Error "Formatting check failed!"; exit 1 }

cargo clippy --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { Write-Error "Clippy linter failed!"; exit 1 }

cargo build --workspace --release
if ($LASTEXITCODE -ne 0) { Write-Error "Cargo build failed!"; exit 1 }

Write-Host "Build completed successfully!" -ForegroundColor Green
