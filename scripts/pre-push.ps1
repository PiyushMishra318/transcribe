# Fast checks before git push (fmt + clippy).
# Usage: powershell -ExecutionPolicy Bypass -File scripts/pre-push.ps1

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

Push-Location $Root
try {
    Write-Host "==> pre-push: cargo fmt --check" -ForegroundColor Cyan
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw "cargo fmt --check failed (exit $LASTEXITCODE)" }

    Write-Host "==> pre-push: cargo clippy" -ForegroundColor Cyan
    cargo clippy --locked --no-default-features -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "cargo clippy failed (exit $LASTEXITCODE)" }

    Write-Host "==> pre-push: OK" -ForegroundColor Green
}
finally {
    Pop-Location
}
