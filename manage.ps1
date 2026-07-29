$ErrorActionPreference = 'Stop'

$command = if ($args.Length -gt 0) { $args[0] } else { 'install' }
$rest = if ($args.Length -gt 1) { $args[1..($args.Length - 1)] } else { @() }
$channel = if ($env:PLUMB_CHANNEL) { $env:PLUMB_CHANNEL } else { 'stable' }
$version = if ($env:PLUMB_VERSION) { $env:PLUMB_VERSION } else { '' }
$publicUrl = if ($env:PLUMB_RELEASES_PUBLIC_URL) { $env:PLUMB_RELEASES_PUBLIC_URL } else { 'https://releases.plumb.perish.uk' }
$installRoot = if ($env:PLUMB_INSTALL_ROOT) { $env:PLUMB_INSTALL_ROOT } else { Join-Path $HOME '.local/share/plumb' }
$localBinDir = if ($env:PLUMB_LOCAL_BIN_DIR) { $env:PLUMB_LOCAL_BIN_DIR } else { Join-Path $HOME '.local/bin' }

function Show-Help {
    @'
plumb manager

Usage:
  manage.ps1 -Help
  manage.ps1 install [--channel stable|beta] [--version vX.Y.Z[-beta.N]]
  manage.ps1 update [--channel stable|beta] [--version vX.Y.Z[-beta.N]]
  manage.ps1 uninstall [--version vX.Y.Z[-beta.N]]
'@ | Write-Output
}

if ($command -in @('-h', '-help', '--help', 'help')) {
    Show-Help
    return
}

for ($index = 0; $index -lt $rest.Length; $index++) {
    switch -Regex ($rest[$index]) {
        '^--channel$' {
            if ($index + 1 -ge $rest.Length) { throw '--channel requires a value' }
            $index++; $channel = $rest[$index]; continue
        }
        '^--channel=(.+)$' { $channel = $Matches[1]; continue }
        '^--version$' {
            if ($index + 1 -ge $rest.Length) { throw '--version requires a value' }
            $index++; $version = $rest[$index]; continue
        }
        '^--version=(.+)$' { $version = $Matches[1]; continue }
        '^--public-url$' {
            if ($index + 1 -ge $rest.Length) { throw '--public-url requires a value' }
            $index++; $publicUrl = $rest[$index]; continue
        }
        '^--public-url=(.+)$' { $publicUrl = $Matches[1]; continue }
        '^--install-root$' {
            if ($index + 1 -ge $rest.Length) { throw '--install-root requires a value' }
            $index++; $installRoot = $rest[$index]; continue
        }
        '^--install-root=(.+)$' { $installRoot = $Matches[1]; continue }
        '^--bin-dir$' {
            if ($index + 1 -ge $rest.Length) { throw '--bin-dir requires a value' }
            $index++; $localBinDir = $rest[$index]; continue
        }
        '^--bin-dir=(.+)$' { $localBinDir = $Matches[1]; continue }
        '^(-h|-help|--help|help)$' {
            Show-Help
            return
        }
        default { throw "unknown argument: $($rest[$index])" }
    }
}

if ($channel -notin @('stable', 'beta')) {
    throw "invalid channel: $channel"
}
if ($command -in @('install', 'update') -and $channel -ne 'stable' -and [string]::IsNullOrWhiteSpace($version)) {
    throw "non-stable channel $channel requires an exact version"
}

function Normalize-Version([string]$Value) {
    $normalized = "v$($Value -replace '^v', '')"
    if ($normalized -notmatch '^v[0-9]+\.[0-9]+\.[0-9]+(-beta\.[1-9][0-9]*)?$') {
        throw "invalid plumb version: $Value"
    }
    return $normalized
}

function Install-Plumb {
    $baseUrl = $publicUrl.TrimEnd('/')
    $resolved = $version
    if ([string]::IsNullOrWhiteSpace($resolved)) {
        $metadata = Invoke-RestMethod -Uri "$baseUrl/$channel/latest/metadata.json"
        $resolved = $metadata.releaseVersion
    }
    if ([string]::IsNullOrWhiteSpace($resolved)) {
        throw 'failed to resolve latest plumb version'
    }
    $resolved = Normalize-Version $resolved
    if ($channel -eq 'stable' -and $resolved -match '-') {
        throw "stable channel cannot install prerelease $resolved"
    }
    if ($channel -eq 'beta' -and $resolved -notmatch '-beta\.[1-9][0-9]*$') {
        throw "version $resolved does not belong to beta"
    }
    $archive = 'plumb-x86_64-pc-windows-msvc.zip'
    $tmpdir = Join-Path ([System.IO.Path]::GetTempPath()) ("plumb-" + [System.Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tmpdir | Out-Null
    try {
        $archivePath = Join-Path $tmpdir $archive
        $prefix = "$baseUrl/$channel/versions/$resolved"
        Invoke-WebRequest -Uri "$prefix/$archive" -OutFile $archivePath
        $checksumsPath = Join-Path $tmpdir 'checksums.txt'
        Invoke-WebRequest -Uri "$prefix/checksums.txt" -OutFile $checksumsPath
        $entry = Get-Content $checksumsPath | Where-Object { $_ -match "\s$([regex]::Escape($archive))$" } | Select-Object -First 1
        if ([string]::IsNullOrWhiteSpace($entry)) {
            throw "no checksum for $archive"
        }
        $expected = ($entry -split '\s+')[0].ToLowerInvariant()
        $actual = (Get-FileHash $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actual -ne $expected) {
            throw "checksum mismatch for $archive"
        }
        $versionRoot = Join-Path $installRoot $resolved
        Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $versionRoot
        New-Item -ItemType Directory -Force -Path $versionRoot, $localBinDir | Out-Null
        Expand-Archive -LiteralPath $archivePath -DestinationPath $versionRoot -Force
        Copy-Item -Force (Join-Path $versionRoot 'plumb.exe') (Join-Path $localBinDir 'plumb.exe')
        & (Join-Path $localBinDir 'plumb.exe') --version
        Write-Output "installed plumb to $(Join-Path $localBinDir 'plumb.exe')"
    }
    finally {
        Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $tmpdir
    }
}

function Uninstall-Plumb {
    $bin = Join-Path $localBinDir 'plumb.exe'
    if (![string]::IsNullOrWhiteSpace($version)) {
        $resolved = Normalize-Version $version
        try {
            if ((& $bin --version) -match [regex]::Escape($resolved)) {
                Remove-Item -Force -ErrorAction SilentlyContinue $bin
            }
        }
        catch {}
        Remove-Item -Recurse -Force -ErrorAction SilentlyContinue (Join-Path $installRoot $resolved)
        Write-Output "removed plumb $resolved"
        return
    }
    Remove-Item -Force -ErrorAction SilentlyContinue $bin
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $installRoot
    Write-Output 'removed plumb'
}

switch ($command) {
    'install' { Install-Plumb }
    'update' { Install-Plumb }
    'uninstall' { Uninstall-Plumb }
    default { throw "unknown command: $command" }
}
