$ErrorActionPreference = 'Stop'

$atom = if ($args.Count -ge 1) { $args[0] } else { '.plumb-atom' }
$mode = if ($args.Count -ge 2) { $args[1] } else { 'bootstrap' }
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
    Invoke-WebRequest -UseBasicParsing -Uri 'https://releases.plumb.perish.uk/manage.ps1' -OutFile $manager
    try {
      $channel = Invoke-RestMethod -Uri 'https://releases.plumb.perish.uk/v1/channels/beta.json'
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
  & $manager install --channel beta --version $held --install-root $versions --bin-dir $bin
} else {
  & $manager install --install-root $versions --bin-dir $bin
}
$tool = Join-Path $bin 'plumb.exe'
function Install-Configuration {
  & $tool configuration --help *> $null
  if ($LASTEXITCODE -eq 0) {
    & $tool configuration install
  } else {
    Write-Output 'installed Plumb has no configuration command; retaining its managed depot seat'
  }
}
if ($mode -eq 'bootstrap') {
  Install-Configuration
}

$target = Join-Path $env:RUNNER_TEMP "plumb-atom-$env:PLUMB_BUILD_COMMIT"
$env:CARGO_TARGET_DIR = $target
$env:PLUMB_BUILD_SOURCE = '1'
cargo build --quiet --locked --manifest-path (Join-Path $atom 'Cargo.toml') --bin plumb
Copy-Item (Join-Path $target 'debug/plumb.exe') $tool -Force
$bin | Out-File -FilePath $env:GITHUB_PATH -Append
& $tool --version
if ($mode -eq 'exact') {
  Install-Configuration
}
