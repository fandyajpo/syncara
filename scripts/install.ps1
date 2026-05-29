#!/usr/bin/env pwsh
# ─────────────────────────────────────────────────────────────
#  Syncara — PowerShell installer (Windows)
#  Usage:
#    irm https://syncara.sh/install.ps1 | iex
#
#  Installs the latest release to $env:LOCALAPPDATA\syncara
#  and adds it to the user PATH.
# ─────────────────────────────────────────────────────────────

param(
  [string]$Version = ""
)

$Repo = "anomalyco/syncara"
$InstallDir = "$env:LOCALAPPDATA\syncara"
$BinDir = "$InstallDir\bin"

# ── helpers ──────────────────────────────────────────────────
function Info  { Write-Host "• $args" -ForegroundColor Cyan }
function Warn  { Write-Host "⚠ $args" -ForegroundColor Yellow }
function Err   { Write-Host "✗ $args" -ForegroundColor Red; exit 1 }

# ── resolve version ─────────────────────────────────────────
if (-not $Version) {
  try {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    $Version = $release.tag_name -replace '^v'
  } catch {
    Err "Could not fetch latest release: $_"
  }
}

$Target = "x86_64-pc-windows-msvc"
$Archive = "syncara-$Version-$Target.zip"
$ArchiveUrl = "https://github.com/$Repo/releases/download/v$Version/$Archive"
$ChecksumUrl = "$ArchiveUrl.sha256"

Info "Syncara v$Version — $Target"

# ── temporary directory ─────────────────────────────────────
$TmpDir = "$env:TEMP\syncara-install-$([System.IO.Path]::GetRandomFileName())"
New-Item -ItemType Directory -Force -Path $TmpDir | Out-Null

try {
  # ── download ──
  Info "Downloading $ArchiveUrl"
  Invoke-WebRequest -Uri $ArchiveUrl -OutFile "$TmpDir\$Archive"

  # ── checksum verification ──
  try {
    $ChecksumContent = Invoke-RestMethod -Uri $ChecksumUrl
    $ExpectedHash = $ChecksumContent.Split(' ')[0]
    $ActualHash = (Get-FileHash "$TmpDir\$Archive" -Algorithm SHA256).Hash.ToLower()
    if ($ExpectedHash -ne $ActualHash) {
      Err "Checksum mismatch — expected $ExpectedHash, got $ActualHash"
    }
    Info "Checksum verified"
  } catch {
    Warn "Could not verify checksum: $_"
  }

  # ── extract ──
  Expand-Archive -Path "$TmpDir\$Archive" -DestinationPath $TmpDir

  # ── install ──
  New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
  Move-Item -Force "$TmpDir\syncara.exe" "$BinDir\syncara.exe"
  Info "Installed to $BinDir\syncara.exe"

  # ── add to PATH ──
  $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
  if ($UserPath -notlike "*$BinDir*") {
    $NewPath = "$UserPath;$BinDir"
    [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
    $env:Path = [Environment]::GetEnvironmentVariable("Path", "User")
    Info "Added $BinDir to user PATH (restart terminal for changes)"
  }

  # ── verify ──
  & "$BinDir\syncara.exe" --version
} finally {
  Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
}
