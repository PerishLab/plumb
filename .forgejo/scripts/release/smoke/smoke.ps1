$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))))
$version = if ($args.Length -gt 0) { $args[0] } else { throw 'missing release version' }
$channel = if ($args.Length -gt 1) { $args[1] } else { 'stable' }
$tmpdir = Join-Path ([System.IO.Path]::GetTempPath()) ("plumb-smoke-" + [System.Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmpdir | Out-Null

try {
    $env:PLUMB_INSTALL_ROOT = Join-Path $tmpdir 'install'
    $env:PLUMB_LOCAL_BIN_DIR = Join-Path $tmpdir 'bin'
    $help = & (Join-Path $root 'manage.ps1') --help | Out-String
    if ($help -notmatch 'Usage:') { throw 'top-level manager help is absent' }
    $help = & (Join-Path $root 'manage.ps1') install --help | Out-String
    if ($help -notmatch 'Usage:') { throw 'command manager help is absent' }
    & (Join-Path $root 'manage.ps1') install --channel $channel --version $version
    $bin = Join-Path $env:PLUMB_LOCAL_BIN_DIR 'plumb.exe'
    & $bin --version
    if ($LASTEXITCODE -ne 0) { throw 'installed plumb --version failed' }
    & $bin doctor $root
    if ($LASTEXITCODE -ne 0) { throw 'installed plumb doctor failed' }
    & (Join-Path $root 'manage.ps1') update --channel $channel --version $version
    & $bin doctor $root
    if ($LASTEXITCODE -ne 0) { throw 'updated plumb doctor failed' }
    & (Join-Path $root 'manage.ps1') uninstall --version $version
    if (Test-Path (Join-Path $env:PLUMB_INSTALL_ROOT $version)) {
        throw "version uninstall left $(Join-Path $env:PLUMB_INSTALL_ROOT $version)"
    }
    if ($env:SMOKE_LATEST -eq '1') {
        if ($channel -eq 'stable') {
            & (Join-Path $root 'manage.ps1') install --channel $channel
            & $bin doctor $root
            if ($LASTEXITCODE -ne 0) { throw 'latest plumb doctor failed' }
            & (Join-Path $root 'manage.ps1') uninstall
            if (Test-Path $env:PLUMB_INSTALL_ROOT) {
                throw "full uninstall left $env:PLUMB_INSTALL_ROOT"
            }
        }
        else {
            $refused = $false
            try {
                & (Join-Path $root 'manage.ps1') install --channel $channel
            }
            catch {
                $refused = $true
            }
            if (!$refused) {
                throw "manager accepted floating $channel install"
            }
        }
    }
}
finally {
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $tmpdir
}
exit 0
