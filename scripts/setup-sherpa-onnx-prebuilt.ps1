# Download and extract sherpa-onnx shared prebuilt libs into target/sherpa-onnx-prebuilt/.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$lock = Get-Content Cargo.lock -Raw
if ($lock -match 'name = "sherpa-onnx-sys"[\s\S]*?version = "([^"]+)"') {
    $Version = $Matches[1]
} else {
    throw "Could not read sherpa-onnx-sys version from Cargo.lock"
}

$TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { "target" }
$CacheRoot = Join-Path $TargetDir "sherpa-onnx-prebuilt"
$Archive = "sherpa-onnx-v$Version-win-x64-shared-MT-Release-lib.tar.bz2"
$Stem = $Archive -replace '\.tar\.bz2$', ''
$LibDir = Join-Path (Join-Path $CacheRoot $Stem) "lib"

if (Test-Path $LibDir) {
    Write-Host "sherpa-onnx prebuilt already present: $LibDir"
} else {
    New-Item -ItemType Directory -Force -Path $CacheRoot | Out-Null
    $Url = "https://github.com/k2-fsa/sherpa-onnx/releases/download/v$Version/$Archive"
    $ArchivePath = Join-Path $CacheRoot $Archive
    Write-Host "Downloading $Url"
    curl.exe -fsSL -o $ArchivePath $Url
    tar -xjf $ArchivePath -C $CacheRoot
    if (-not (Test-Path $LibDir)) {
        throw "Expected lib directory missing after extract: $LibDir"
    }
    Write-Host "Extracted to $(Join-Path $CacheRoot $Stem)"
}

$env:SHERPA_ONNX_LIB_DIR = $LibDir
if ($env:GITHUB_ENV) {
    Add-Content -Path $env:GITHUB_ENV -Value "SHERPA_ONNX_LIB_DIR=$LibDir"
}
Write-Host "SHERPA_ONNX_LIB_DIR=$LibDir"
