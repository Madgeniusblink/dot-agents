# Install .agents/ to DOT_AGENTS_HOME. Updates upstream-owned files.
# Never overwrites existing memory/ or decisions/ files.
param([switch]$Force)

$ErrorActionPreference = "Stop"

if ($args.Count -gt 0) {
  throw "unknown argument: $($args[0])"
}

function Get-AgentHome {
  if ($env:DOT_AGENTS_HOME) { return $env:DOT_AGENTS_HOME }
  if ($env:HOME) { return Join-Path $env:HOME ".agents" }
  if ($env:USERPROFILE) { return Join-Path $env:USERPROFILE ".agents" }
  throw "set DOT_AGENTS_HOME or HOME"
}

function Test-UserOwned([string]$Rel) {
  return ($Rel -eq "memory") -or ($Rel.StartsWith("memory\")) -or ($Rel.StartsWith("memory/")) `
    -or ($Rel -eq "decisions") -or ($Rel.StartsWith("decisions\")) -or ($Rel.StartsWith("decisions/"))
}

function Test-ShouldCopy([string]$Rel, [string]$Target) {
  if (-not (Test-Path $Target)) { return $true }
  if (Test-UserOwned $Rel) { return $false }
  return $true
}

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$Src = Join-Path $Root ".agents"
$Dest = Get-AgentHome

if (-not (Test-Path $Src)) { throw "missing $Src" }
New-Item -ItemType Directory -Force -Path $Dest | Out-Null

Get-ChildItem -Path $Src -Recurse -File | ForEach-Object {
  $rel = $_.FullName.Substring($Src.Length + 1).Replace("\", "/")
  $target = Join-Path $Dest $rel
  if (Test-ShouldCopy $rel $target) {
    $dir = Split-Path $target -Parent
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    Copy-Item -Path $_.FullName -Destination $target -Force
  }
}

Write-Host "Installed (upstream refreshed; memory/ and decisions/ left unchanged) -> $Dest"
