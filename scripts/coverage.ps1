# PowerShell Coverage Generation Script
Write-Host "Generating code coverage report (target >= 90%)..." -ForegroundColor Cyan

cargo llvm-cov --workspace --all-targets --lcov --output-path target/coverage/lcov.info --html --output-dir target/coverage/html
if ($LASTEXITCODE -ne 0) {
    Write-Warning "cargo-llvm-cov not installed or errored. Run: cargo install cargo-llvm-cov"
} else {
    Write-Host "Coverage report generated at target/coverage/html/index.html" -ForegroundColor Green
}
