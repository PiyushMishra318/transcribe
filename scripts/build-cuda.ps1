# Build episode-transcribe with CUDA (run from repo root or via scripts/).
$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$cuda = $env:CUDA_PATH
if (-not $cuda -or -not (Test-Path $cuda)) {
    $latest = Get-ChildItem "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA" -ErrorAction SilentlyContinue |
        Sort-Object Name -Descending | Select-Object -First 1
    if ($latest) { $cuda = $latest.FullName }
}
if (-not $cuda) { throw "CUDA Toolkit not found. Install: winget install Nvidia.CUDA" }

$env:CUDA_PATH = $cuda
$env:CUDA_HOME = $cuda
$env:CudaToolkitDir = "$cuda\"
if (-not $env:LIBCLANG_PATH) {
    $llvm = "C:\Program Files\LLVM\bin"
    if (Test-Path $llvm) { $env:LIBCLANG_PATH = $llvm }
}
$env:Path = "$cuda\bin;$env:Path"

$vcvars = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if (-not (Test-Path $vcvars)) {
    $vcvars = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
}
if (-not (Test-Path $vcvars)) { throw "Visual Studio C++ Build Tools not found" }

Write-Host "CUDA_PATH=$cuda"
Write-Host "CudaToolkitDir=$env:CudaToolkitDir"
Write-Host "Using: $vcvars"

Push-Location $Root
try {
    cmd /c "`"$vcvars`" >nul && set CUDA_PATH=$cuda&& set CudaToolkitDir=$cuda\&& cargo build --release --locked"
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
} finally {
    Pop-Location
}
