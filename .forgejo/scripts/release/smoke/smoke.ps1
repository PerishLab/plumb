$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))))
$version = if ($args.Length -gt 0) { $args[0] } else { throw 'missing release version' }
$channel = if ($args.Length -gt 1) { $args[1] } else { 'stable' }
$tmpdir = Join-Path ([System.IO.Path]::GetTempPath()) ("plumb-smoke-" + [System.Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmpdir | Out-Null

try {
    $env:PLUMB_INSTALL_ROOT = Join-Path $tmpdir 'install'
    $env:PLUMB_LOCAL_BIN_DIR = Join-Path $tmpdir 'bin'
    & (Join-Path $root 'manage.ps1') install --channel $channel --version $version
    $bin = Join-Path $env:PLUMB_LOCAL_BIN_DIR 'plumb.exe'
    & $bin --version
    & $bin doctor $root
    & (Join-Path $root 'manage.ps1') update --channel $channel --version $version
    & $bin doctor $root
    & (Join-Path $root 'manage.ps1') uninstall --version $version
    if (Test-Path (Join-Path $env:PLUMB_INSTALL_ROOT $version)) {
        throw "version uninstall left $(Join-Path $env:PLUMB_INSTALL_ROOT $version)"
    }
    if ($env:SMOKE_LATEST -eq '1') {
        & (Join-Path $root 'manage.ps1') install --channel $channel
        & $bin doctor $root
        & (Join-Path $root 'manage.ps1') uninstall
        if (Test-Path $env:PLUMB_INSTALL_ROOT) {
            throw "full uninstall left $env:PLUMB_INSTALL_ROOT"
        }
    }
}
finally {
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $tmpdir
}
