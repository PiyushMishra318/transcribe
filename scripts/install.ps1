# Install the `transcribe` CLI for the current user (Windows).
# Usage: powershell -ExecutionPolicy Bypass -File scripts/install.ps1
# Optional: -CpuOnly  for CPU-only build (no CUDA)

param(
    [switch]$CpuOnly
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

Push-Location $Root
try {
    Write-Host "==> Episode Transcribe installer" -ForegroundColor Cyan
    Write-Host "    Source: $Root"

    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Rust (cargo) not found. Install from https://rustup.rs/"
    }

    if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
        Write-Warning "ffmpeg not found on PATH. Install from https://ffmpeg.org/ (required at runtime)."
    }

    Write-Host "==> Building release binary..."
    if ($CpuOnly) {
        cargo build --release --locked --no-default-features
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit $LASTEXITCODE)" }
    } else {
        $buildCuda = Join-Path $PSScriptRoot "build-cuda.ps1"
        if (-not (Test-Path $buildCuda)) { throw "missing scripts/build-cuda.ps1" }
        & $buildCuda
    }

    $releaseDir = Join-Path $Root "target\release"
    $built = Join-Path $releaseDir "transcribe.exe"
    if (-not (Test-Path $built)) {
        throw "Build failed: $built not found"
    }

    $installDir = Join-Path $env:LOCALAPPDATA "Programs\episode-transcribe"
    New-Item -ItemType Directory -Force -Path $installDir | Out-Null
    $dest = Join-Path $installDir "transcribe.exe"
    Copy-Item $built $dest -Force

    # sherpa-onnx shared runtime DLLs (required on Windows)
    Get-ChildItem $releaseDir -Filter "*.dll" | ForEach-Object {
        Copy-Item $_.FullName $installDir -Force
    }

    if (-not $CpuOnly) {
        $cudaBin = $env:CUDA_PATH
        if (-not $cudaBin) {
            $latest = Get-ChildItem "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA" -ErrorAction SilentlyContinue |
                Sort-Object Name -Descending | Select-Object -First 1
            if ($latest) { $cudaBin = $latest.FullName }
        }
        $cudaX64 = Join-Path $cudaBin "bin\x64"
        if (Test-Path $cudaX64) {
            $cudaDlls = @("cudart64_*.dll", "cublas64_*.dll", "cublasLt64_*.dll")
            foreach ($pat in $cudaDlls) {
                Get-ChildItem (Join-Path $cudaX64 $pat) -ErrorAction SilentlyContinue | ForEach-Object {
                    Copy-Item $_.FullName $installDir -Force
                }
            }
            Write-Host "==> Copied CUDA runtime DLLs from $cudaX64" -ForegroundColor Green
        }
    }

    $modelsDir = Join-Path $installDir "models"
    New-Item -ItemType Directory -Force -Path $modelsDir | Out-Null
    $modelsReadme = Join-Path $Root "models\README.md"
    if (Test-Path $modelsReadme) {
        Copy-Item $modelsReadme (Join-Path $modelsDir "README.md") -Force
    }
    Write-Host "==> Models directory: $modelsDir" -ForegroundColor Green

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$installDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
        $env:Path = "$env:Path;$installDir"
        Write-Host "==> Added to user PATH: $installDir" -ForegroundColor Green
        Write-Host "    Open a new terminal, then run: transcribe --version"
    } else {
        Write-Host "==> Already on PATH: $installDir" -ForegroundColor Green
    }

    & $dest --version
    Write-Host "==> Installed successfully." -ForegroundColor Green
    Write-Host @"

Next steps:
  1. Download models — see models/README.md
  2. cd into your episode folder (where .mp4 files live)
  3. transcribe project init my-campaign
  4. transcribe profiles build && transcribe profiles label
  5. transcribe run .

Help:  transcribe help
Remove: transcribe uninstall -y  (or scripts/uninstall.ps1)

"@ -ForegroundColor DarkGray
} finally {
    Pop-Location
}
