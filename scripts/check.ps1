# Run the same checks as CI locally (fmt + clippy + test).
# Usage: powershell -ExecutionPolicy Bypass -File scripts/check.ps1

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

Push-Location $Root
try {
    Write-Host "==> check: cargo fmt --check" -ForegroundColor Cyan
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw "cargo fmt --check failed (exit $LASTEXITCODE)" }

    Write-Host "==> check: cargo clippy" -ForegroundColor Cyan
    cargo clippy --locked --no-default-features -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "cargo clippy failed (exit $LASTEXITCODE)" }

    Write-Host "==> check: cargo test" -ForegroundColor Cyan
    cargo test --locked --no-default-features
    if ($LASTEXITCODE -ne 0) { throw "cargo test failed (exit $LASTEXITCODE)" }

    Write-Host "==> check: OK" -ForegroundColor Green
}
finally {
    Pop-Location
}
