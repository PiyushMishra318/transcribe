# Uninstall transcribe (Windows). Same as: transcribe uninstall -y [--purge]
param(
    [switch]$Purge,
    [switch]$Yes
)

$ErrorActionPreference = "Stop"
$args = @("uninstall", "-y")
if ($Purge) { $args += "--purge" }

$installDir = Join-Path $env:LOCALAPPDATA "Programs\episode-transcribe"
$exe = Join-Path $installDir "transcribe.exe"

if (Test-Path $exe) {
    & $exe @args
    exit $LASTEXITCODE
}

# Fallback if binary already removed
Write-Host "==> transcribe.exe not found; cleaning install dir and PATH manually..."
if (Test-Path $installDir) {
    Remove-Item -Recurse -Force $installDir
    Write-Host "removed: $installDir"
}

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -like "*episode-transcribe*") {
    $parts = $userPath -split ';' | Where-Object { $_ -notmatch 'episode-transcribe' }
    $newPath = ($parts | Where-Object { $_ }) -join ';'
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "updated user PATH"
}

if ($Purge) {
    $home = Join-Path $env:USERPROFILE ".transcribe"
    if (Test-Path $home) {
        Remove-Item -Recurse -Force $home
        Write-Host "removed: $home"
    }
}

if (-not $Yes) {
    Write-Host "done (used fallback cleanup)"
}
