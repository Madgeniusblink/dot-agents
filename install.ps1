# Install .agents/ to DOT_AGENTS_HOME.
param()

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

function Test-SkipMeta([string]$Rel) {
  return ($Rel -eq ".user-owned") -or ($Rel -eq ".managed-refresh")
}

function Get-RuleLines([string]$Path) {
  if (-not (Test-Path $Path)) { return @() }
  Get-Content $Path | ForEach-Object {
    $line = ($_ -split '#')[0].Trim()
    if ($line) { $line }
  }
}

function Test-MatchesRuleLine([string]$Rel, [string]$Line) {
  if ($Line.EndsWith("/")) {
    $prefix = $Line.TrimEnd("/")
    return ($Rel -eq $prefix) -or ($Rel.StartsWith("$prefix/"))
  }
  return $Rel -eq $Line
}

function Test-MatchesList([string]$Rel, [string[]]$Lines) {
  foreach ($line in $Lines) {
    if (Test-MatchesRuleLine $Rel $line) { return $true }
  }
  return $false
}

function Test-UserOwned([string]$Rel, [string[]]$UserLines) {
  Test-MatchesList $Rel $UserLines
}

function Test-ManagedRefresh([string]$Rel, [string[]]$ManagedLines) {
  Test-MatchesList $Rel $ManagedLines
}

function Test-ShouldCopy([string]$Rel, [string]$Target, [string[]]$UserLines, [string[]]$ManagedLines) {
  if (-not (Test-Path $Target)) { return $true }
  if (Test-UserOwned $Rel $UserLines) { return $false }
  if (Test-ManagedRefresh $Rel $ManagedLines) { return $true }
  return $false
}

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$Src = Join-Path $Root ".agents"
$Dest = Get-AgentHome
$userLines = Get-RuleLines (Join-Path $Src ".user-owned")
$managedLines = Get-RuleLines (Join-Path $Src ".managed-refresh")

if (-not (Test-Path $Src)) { throw "missing $Src" }
New-Item -ItemType Directory -Force -Path $Dest | Out-Null

Get-ChildItem -Path $Src -Recurse -File | ForEach-Object {
  $rel = $_.FullName.Substring($Src.Length + 1).Replace("\", "/")
  if (Test-SkipMeta $rel) { return }
  $target = Join-Path $Dest $rel
  if (Test-ShouldCopy $rel $target $userLines $managedLines) {
    $dir = Split-Path $target -Parent
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    Copy-Item -Path $_.FullName -Destination $target -Force
  }
}

Write-Host "Installed (missing added; managed paths refreshed; user-owned unchanged) -> $Dest"
