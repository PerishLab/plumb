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

$manager = Join-Path $env:RUNNER_TEMP 'manage-plumb.ps1'
$held = $null
for ($attempt = 1; $attempt -le 5; $attempt++) {
  try {
    Invoke-WebRequest -UseBasicParsing -TimeoutSec 30 -Uri 'https://releases.plumb.perish.uk/manage.ps1' -OutFile $manager
    try {
      $channel = Invoke-RestMethod -TimeoutSec 30 -Uri 'https://releases.plumb.perish.uk/v1/channels/stable.json'
      $held = $channel.releaseVersion
    } catch {
      $held = $null
    }
    break
  } catch {
    if ($attempt -eq 5) { throw }
    Start-Sleep -Seconds $attempt
  }
}

$seat = if ($held) {
  Join-Path $env:RUNNER_TEMP "plumb-$held"
} else {
  Join-Path $env:RUNNER_TEMP 'plumb-bootstrap'
}
$versions = Join-Path $seat 'versions'
$bin = Join-Path $seat 'bin'
if ($held) {
  & $manager install --channel stable --version $held --install-root $versions --bin-dir $bin
} else {
  & $manager install --install-root $versions --bin-dir $bin
}
$tool = Join-Path $bin 'plumb.exe'
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
if ($mode -eq 'bootstrap') {
  Install-Configuration
}

$target = Join-Path $env:RUNNER_TEMP "plumb-atom-$env:PLUMB_BUILD_COMMIT"
$archive = Join-Path $env:RUNNER_TEMP "plumb-atom-$env:PLUMB_BUILD_COMMIT.tgz"
$hostTarget = ((rustc -vV | Select-String '^host: ').Line -replace '^host: ', '')
$compiler = rustc --version
function Install-AtomSource([string]$uri) {
  $match = [regex]::Match($uri, '/workloads/([0-9a-fA-F]{64})\.tgz$')
  if (-not $match.Success) { throw "invalid Plumb atom workload URL: $uri" }
  Invoke-WebRequest -UseBasicParsing -TimeoutSec 300 -Uri $uri -OutFile $archive
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
$supportsWorkload = & $tool workflow plan --help 2>&1 | Select-String -SimpleMatch '--workload'
if ($source) {
  Install-AtomSource $source
} elseif (-not [string]::IsNullOrWhiteSpace($env:PLUMB_WORKFLOW_INVENTORY_URL) -and $supportsWorkload) {
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
Copy-Item (Join-Path $target 'debug/plumb.exe') $tool -Force
$bin | Out-File -FilePath $env:GITHUB_PATH -Append
& $tool --version
if ($mode -eq 'exact') {
  $env:PLUMB_HOME = Join-Path $env:RUNNER_TEMP "plumb-home-$configuration"
  Install-Configuration (Join-Path $env:PLUMB_HOME 'configurations')
  "PLUMB_HOME=$env:PLUMB_HOME" | Out-File -FilePath $env:GITHUB_ENV -Append
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
    Copy-Item (Join-Path $target 'debug/plumb.exe') $tool -Force
    Write-Output "accepted exact Plumb atom inventory winner $source for $hostTarget"
  } else {
    $digest = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
    $source = "$($env:PLUMB_WORKFLOW_INVENTORY_URL.TrimEnd('/'))/workloads/$digest.tgz"
  }
}
if ($source -and $env:GITHUB_OUTPUT) {
  "source=$source" | Out-File -FilePath $env:GITHUB_OUTPUT -Append
}
if ($source -and $env:GITHUB_ENV) {
  "PLUMB_ATOM_SOURCE=$source" | Out-File -FilePath $env:GITHUB_ENV -Append
}
