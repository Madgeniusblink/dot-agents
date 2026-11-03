# Install .agents/ to DOT_AGENTS_HOME.
# Replaces a file only if it is unchanged since install wrote it (.installed manifest).
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

function Get-Hash([string]$Path) {
  (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLower()
}

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$Src = Join-Path $Root ".agents"
$Dest = Get-AgentHome

if (-not (Test-Path $Src)) { throw "missing $Src" }
New-Item -ItemType Directory -Force -Path $Dest | Out-Null

# Manifest line: <sha256>  <relative path>
$manifestPath = Join-Path $Dest ".installed"
$recorded = @{}
if (Test-Path $manifestPath) {
  Get-Content $manifestPath | ForEach-Object {
    if ($_.Length -gt 66) { $recorded[$_.Substring(66)] = $_.Substring(0, 64) }
  }
}
$next = [System.Collections.Generic.List[string]]::new()
$shipped = @{}

Get-ChildItem -Path $Src -Recurse -File | Sort-Object FullName | ForEach-Object {
  $rel = $_.FullName.Substring($Src.Length + 1).Replace("\", "/")
  $shipped[$rel] = $true
  $target = Join-Path $Dest $rel
  $new = Get-Hash $_.FullName
  if (-not (Test-Path $target)) {
    $dir = Split-Path $target -Parent
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    Copy-Item -Path $_.FullName -Destination $target
    $next.Add("$new  $rel")
    return
  }
  $have = Get-Hash $target
  $old = $recorded[$rel]
  if ($have -eq $new) {
    $next.Add("$new  $rel")
  } elseif ($old -and $have -eq $old) {
    Copy-Item -Path $_.FullName -Destination $target -Force
    $next.Add("$new  $rel")
    Write-Host "updated: $rel"
  } else {
    # Edited, or present before the manifest existed. Never guess: keep it.
    if ($old) { $next.Add("$old  $rel") }
    Write-Host "kept (edited): $rel"
  }
}

# Files install wrote that the template no longer ships: remove if unchanged.
foreach ($rel in $recorded.Keys) {
  if ($shipped.ContainsKey($rel)) { continue }
  $target = Join-Path $Dest $rel
  if (-not (Test-Path $target)) { continue }
  if ((Get-Hash $target) -eq $recorded[$rel]) {
    Remove-Item $target
    Write-Host "removed: $rel"
  } else {
    Write-Host "kept (edited, no longer shipped): $rel"
  }
}

Set-Content -Path $manifestPath -Value $next
Write-Host "Installed -> $Dest"
Write-Host "A kept file gets no updates. Delete one you never edited to take the current version."
