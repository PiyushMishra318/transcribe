# Download and extract sherpa-onnx shared prebuilt libs into target/sherpa-onnx-prebuilt/.
# SHERPA_ONNX_LIB_DIR must be absolute: dependency build.rs resolves relative paths from the crate dir.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$lock = Get-Content Cargo.lock -Raw
if ($lock -match 'name = "sherpa-onnx-sys"[\s\S]*?version = "([^"]+)"') {
    $Version = $Matches[1]
} else {
    throw "Could not read sherpa-onnx-sys version from Cargo.lock"
}

if ($env:CARGO_TARGET_DIR) {
    $TargetDir = if ([System.IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
        $env:CARGO_TARGET_DIR
    } else {
        Join-Path $Root $env:CARGO_TARGET_DIR
    }
} else {
    $TargetDir = Join-Path $Root "target"
}

$CacheRoot = Join-Path $TargetDir "sherpa-onnx-prebuilt"
$Archive = "sherpa-onnx-v$Version-win-x64-shared-MT-Release-lib.tar.bz2"
$Stem = $Archive -replace '\.tar\.bz2$', ''
$LibDir = Join-Path (Join-Path $CacheRoot $Stem) "lib"

function Test-SherpaLibReady([string]$Dir) {
    if (-not (Test-Path -PathType Container $Dir)) { return $false }
    return @(Get-ChildItem -Path $Dir -Filter "*.dll" -File -ErrorAction SilentlyContinue).Count -gt 0
}

if (Test-SherpaLibReady $LibDir) {
    Write-Host "sherpa-onnx prebuilt already present: $LibDir"
} else {
    Write-Host "Installing sherpa-onnx prebuilt ($Stem)"
    $StemDir = Join-Path $CacheRoot $Stem
    if (Test-Path $StemDir) { Remove-Item -Recurse -Force $StemDir }
    New-Item -ItemType Directory -Force -Path $CacheRoot | Out-Null
    $Url = "https://github.com/k2-fsa/sherpa-onnx/releases/download/v$Version/$Archive"
    $ArchivePath = Join-Path $CacheRoot $Archive
    Write-Host "Downloading $Url"
    curl.exe -fsSL -o $ArchivePath $Url
    tar -xjf $ArchivePath -C $CacheRoot
    if (-not (Test-SherpaLibReady $LibDir)) {
        throw "Expected lib directory missing or empty after extract: $LibDir"
    }
    Write-Host "Extracted to $(Join-Path $CacheRoot $Stem)"
}

$LibDir = (Resolve-Path $LibDir).Path

if (-not (Test-SherpaLibReady $LibDir)) {
    throw "sherpa lib directory is not ready: $LibDir"
}

Write-Host "sherpa-onnx libs in ${LibDir}:"
Get-ChildItem $LibDir

$env:SHERPA_ONNX_LIB_DIR = $LibDir
if ($env:GITHUB_ENV) {
    Add-Content -Path $env:GITHUB_ENV -Value "SHERPA_ONNX_LIB_DIR=$LibDir"
    $pathEntry = "PATH=$LibDir"
    if ($env:PATH) { $pathEntry = "PATH=$LibDir;$env:PATH" }
    Add-Content -Path $env:GITHUB_ENV -Value $pathEntry
}
Write-Host "SHERPA_ONNX_LIB_DIR=$LibDir"
