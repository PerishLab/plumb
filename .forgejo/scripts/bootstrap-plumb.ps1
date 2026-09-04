$ErrorActionPreference = 'Stop'

$atom = if ($args.Count -ge 1) { $args[0] } else { '.plumb-atom' }
$mode = if ($args.Count -ge 2) { $args[1] } else { 'bootstrap' }
$configuration = if ($args.Count -ge 3) { $args[2] } else { $env:PLUMB_BUILD_VERSION }
if ($mode -notin @('bootstrap', 'exact')) {
  throw "unknown Plumb bootstrap mode: $mode"
}

foreach ($name in @('PLUMB_BUILD_VERSION', 'PLUMB_BUILD_COMMIT')) {
  if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name))) { throw "$name is required" }
}
if ([string]::IsNullOrWhiteSpace($env:PLUMB_BUILD_CHANNEL)) {
  $env:PLUMB_BUILD_CHANNEL = switch -Regex ($env:PLUMB_BUILD_VERSION) {
    '-alpha\.' { 'alpha'; break }
    '-beta\.' { 'beta'; break }
    '-rc\.' { 'rc'; break }
    default { 'stable' }
  }
}

$target = Join-Path $env:RUNNER_TEMP "plumb-atom-$env:PLUMB_BUILD_COMMIT"
$archive = Join-Path $env:RUNNER_TEMP "plumb-atom-$env:PLUMB_BUILD_COMMIT.tgz"
$bin = Join-Path $env:RUNNER_TEMP "plumb-exact-$env:PLUMB_BUILD_COMMIT/bin"
$tool = Join-Path $bin 'plumb.exe'
$hostTarget = ((rustc -vV | Select-String '^host: ').Line -replace '^host: ', '')
$compiler = rustc --version
function Install-Configuration([string]$Path = '') {
  & $tool configuration --help *> $null
  if ($LASTEXITCODE -eq 0) {
    if ([string]::IsNullOrWhiteSpace($configuration)) { throw 'Plumb configuration version is required' }
    if ([string]::IsNullOrWhiteSpace($Path)) {
      & $tool configuration install --version $configuration
    } else {
      & $tool configuration install --version $configuration --path $Path
    }
  } else {
    Write-Output 'installed Plumb has no configuration command; retaining its managed depot seat'
  }
}
function Install-AtomSource([string]$uri) {
  $match = [regex]::Match($uri, '/workloads/([0-9a-fA-F]{64})\.tgz$')
  if (-not $match.Success) { throw "invalid Plumb atom workload URL: $uri" }
  for ($attempt = 1; $attempt -le 30; $attempt++) {
    try {
      Invoke-WebRequest -UseBasicParsing -TimeoutSec 300 -Uri $uri -OutFile $archive
      break
    } catch {
      if ($attempt -eq 30) { throw }
      Start-Sleep -Seconds 2
    }
  }
  $actual = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
  if ($actual -ne $match.Groups[1].Value.ToLowerInvariant()) { throw 'Plumb atom workload digest mismatch' }
  $debug = Join-Path $target 'debug'
  New-Item -ItemType Directory -Force -Path $debug | Out-Null
  tar -xzf $archive -C $debug
}
function Get-AtomPlan {
  & $tool workflow plan `
    --world "target=$hostTarget" `
    --world "version=$env:PLUMB_BUILD_VERSION" `
    --world "channel=$env:PLUMB_BUILD_CHANNEL" `
    --workload "commit=$env:PLUMB_BUILD_COMMIT" `
    --world "compiler=$compiler" `
    --world 'profile=debug' `
    --root 'ship/atom=*' `
    --inventory-url $env:PLUMB_WORKFLOW_INVENTORY_URL `
    $atom | ConvertFrom-Json
}
$keys = $null
$source = $env:PLUMB_ATOM_SOURCE
$handoff = $env:PLUMB_ATOM_HANDOFF
$inventoryBase = $env:PLUMB_WORKFLOW_INVENTORY_URL.TrimEnd('/') -replace '/inventory\.json$', ''
if (-not $source -and $handoff) {
  $reuse = $handoff | ConvertFrom-Json
  if ($reuse.type -ne 'workload' -or -not $reuse.source) { throw 'invalid Plumb atom handoff' }
  $source = $reuse.source
}
$supportsWorkload = $null
if (-not $source) {
  $manager = Join-Path $env:RUNNER_TEMP 'manage-plumb.ps1'
  $managerReady = $false
  for ($attempt = 1; $attempt -le 5; $attempt++) {
    try {
      Invoke-WebRequest -UseBasicParsing -TimeoutSec 30 -Uri 'https://releases.plumb.perish.uk/manage.ps1' -OutFile $manager
      $managerReady = $true
      break
    } catch {
      if ($attempt -lt 5) { Start-Sleep -Seconds $attempt }
    }
  }
  if ($managerReady) {
    try {
      $channel = Invoke-RestMethod -TimeoutSec 30 -Uri 'https://releases.plumb.perish.uk/v1/channels/stable.json'
      $held = $channel.releaseVersion
    } catch {
      $held = $null
    }
    $seat = Join-Path $env:RUNNER_TEMP "plumb-bootstrap-$(if ($held) { $held } else { 'stable' })"
    $versions = Join-Path $seat 'versions'
    $bootstrapBin = Join-Path $seat 'bin'
    if ($held) {
      & $manager install --channel stable --version $held --install-root $versions --bin-dir $bootstrapBin
    } else {
      & $manager install --install-root $versions --bin-dir $bootstrapBin
    }
    if ($LASTEXITCODE -eq 0) {
      $tool = Join-Path $bootstrapBin 'plumb.exe'
      Write-Output "installed stable Plumb $(if ($held) { $held } else { '' }) for atom planning"
    } else {
      Write-Output 'stable Plumb is unavailable; cold-building the exact atom'
    }
  } else {
    Write-Output 'stable Plumb manager is unavailable; cold-building the exact atom'
  }
}
if (Test-Path $tool) {
  $supportsWorkload = & $tool workflow plan --help 2>&1 | Select-String -SimpleMatch '--workload'
}
if (-not $source -and -not [string]::IsNullOrWhiteSpace($env:PLUMB_WORKFLOW_INVENTORY_URL) -and $supportsWorkload) {
  $plan = Get-AtomPlan
  $action = $plan.actions | Where-Object { $_.name -eq 'ship/atom' }
  $keys = $action.keys | ConvertTo-Json -Compress
  if ($action.decision -eq 'reuse' -and $action.reuse.type -eq 'workload') {
    $source = $action.reuse.source
  }
}
if ($source) {
  Install-AtomSource $source
  Write-Output "reused exact Plumb atom $env:PLUMB_BUILD_COMMIT for $hostTarget"
} else {
  $env:CARGO_TARGET_DIR = $target
  $env:PLUMB_BUILD_SOURCE = '1'
  cargo build --quiet --locked --manifest-path (Join-Path $atom 'Cargo.toml') --bin plumb
  tar -czf $archive -C (Join-Path $target 'debug') plumb.exe
  Write-Output "built exact Plumb atom $env:PLUMB_BUILD_COMMIT for $hostTarget"
}
New-Item -ItemType Directory -Force -Path $bin | Out-Null
Copy-Item (Join-Path $target 'debug/plumb.exe') (Join-Path $bin 'plumb.exe') -Force
$tool = Join-Path $bin 'plumb.exe'
$bin | Out-File -FilePath $env:GITHUB_PATH -Append
& $tool --version
if ($mode -eq 'exact') {
  $env:PLUMB_HOME = Join-Path $env:RUNNER_TEMP "plumb-home-$configuration"
  Install-Configuration (Join-Path $env:PLUMB_HOME 'configurations')
  "PLUMB_HOME=$env:PLUMB_HOME" | Out-File -FilePath $env:GITHUB_ENV -Append
} else {
  Install-Configuration
}
if (-not $keys -and -not [string]::IsNullOrWhiteSpace($env:PLUMB_WORKFLOW_INVENTORY_URL)) {
  $plan = Get-AtomPlan
  $action = $plan.actions | Where-Object { $_.name -eq 'ship/atom' }
  $keys = $action.keys | ConvertTo-Json -Compress
}
if (-not $source -and $keys) {
  & $tool workflow record ship/atom --keys $keys --workload $archive
  if ($LASTEXITCODE -ne 0) {
    $winner = $null
    for ($attempt = 1; $attempt -le 12; $attempt++) {
      $raced = Get-AtomPlan
      $winner = $raced.actions | Where-Object {
        $_.name -eq 'ship/atom' -and $_.decision -eq 'reuse' -and $_.reuse.type -eq 'workload'
      }
      if ($winner) { break }
      if ($attempt -lt 12) {
        Write-Output "waiting for exact Plumb atom inventory winner ($attempt/12)"
        Start-Sleep -Seconds 5
      }
    }
    if (-not $winner) { throw 'cannot resolve exact Plumb atom inventory race' }
    $source = $winner.reuse.source
    Install-AtomSource $source
    Copy-Item (Join-Path $target 'debug/plumb.exe') (Join-Path $bin 'plumb.exe') -Force
    Write-Output "accepted exact Plumb atom inventory winner $source for $hostTarget"
  } else {
    $workloadDigest = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
    $source = "$inventoryBase/workloads/$workloadDigest.tgz"
    Install-AtomSource $source
    Copy-Item (Join-Path $target 'debug/plumb.exe') (Join-Path $bin 'plumb.exe') -Force
    Write-Output "confirmed exact Plumb atom visibility $source for $hostTarget"
  }
}
if ($source -and $env:GITHUB_OUTPUT) {
  "source=$source" | Out-File -FilePath $env:GITHUB_OUTPUT -Append
}
if ($source -and $env:GITHUB_ENV) {
  "PLUMB_ATOM_SOURCE=$source" | Out-File -FilePath $env:GITHUB_ENV -Append
}
