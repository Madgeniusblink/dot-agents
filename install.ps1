# Install .agents/ to DOT_AGENTS_HOME (default %USERPROFILE%\.agents). Skips existing files unless -Force.
param([switch]$Force)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$Src = Join-Path $Root ".agents"
$Dest = if ($env:DOT_AGENTS_HOME) { $env:DOT_AGENTS_HOME } else { Join-Path $env:USERPROFILE ".agents" }

if (-not (Test-Path $Src)) { throw "missing $Src" }
New-Item -ItemType Directory -Force -Path $Dest | Out-Null

Get-ChildItem -Path $Src -Recurse -File | ForEach-Object {
    $rel = $_.FullName.Substring($Src.Length + 1)
    if ($rel -eq "STRUCTURE.md") { return }
    $target = Join-Path $Dest $rel
    $dir = Split-Path $target -Parent
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    if ($Force -or -not (Test-Path $target)) {
        Copy-Item -Path $_.FullName -Destination $target -Force:$Force
    }
}

if ($Force) { Write-Host "Installed (overwrote existing files) -> $Dest" }
else { Write-Host "Installed (skipped existing files; use -Force to overwrite) -> $Dest" }
