param([string]$Manifest = "plumb.toml")

$ErrorActionPreference = "Stop"
if (-not (Test-Path -LiteralPath $Manifest -PathType Leaf)) {
  throw "$Manifest does not exist"
}
$lines = Get-Content -LiteralPath $Manifest
if ($lines | Where-Object { $_ -match '^\[\[document\]\]\s*$' }) {
  throw "$Manifest already declares document bindings"
}

$kept = [System.Collections.Generic.List[string]]::new()
$skip = $false
foreach ($line in $lines) {
  if ($line -match '^\[\[lock\]\]\s*$' -or $line -match '^\[skill\]\s*$') {
    $skip = $true
    continue
  }
  if ($line -match '^\[') {
    $skip = $false
  }
  if (-not $skip) {
    $kept.Add($line)
  }
}

$kept.Add("")
$kept.Add("[[document]]")
$kept.Add('strategy = "agent"')
$kept.Add('source = [{ path = ".", seal = "" }]')
$kept.Add('target-seal = ""')

if (Test-Path -LiteralPath "skills" -PathType Container) {
  foreach ($skill in Get-ChildItem -LiteralPath "skills" -Directory | Sort-Object Name) {
    foreach ($leaf in @("SKILL.md", "PATHS.md", "SCENARIOS.md")) {
      if (-not (Test-Path -LiteralPath (Join-Path $skill.FullName $leaf) -PathType Leaf)) {
        throw "$($skill.FullName) misses $leaf"
      }
    }
    $kept.Add("")
    $kept.Add("[[document]]")
    $kept.Add('strategy = "brief"')
    $kept.Add("name = `"$($skill.Name)`"")
    $kept.Add('source = [{ path = ".", seal = "" }]')
    $kept.Add('target-seal = ""')
  }
}

$draft = "$Manifest.migration-$PID"
[System.IO.File]::WriteAllLines($draft, $kept, [System.Text.UTF8Encoding]::new($false))
Move-Item -LiteralPath $draft -Destination $Manifest -Force
Write-Output "$Manifest now carries unaffirmed document bindings"
