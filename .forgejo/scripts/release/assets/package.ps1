$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))))
$name = 'plumb'
$cargoToml = Join-Path $root 'Cargo.toml'
$version = (Select-String -Path $cargoToml -Pattern '^version = "(.+)"$').Matches[0].Groups[1].Value
$releaseVersion = if ($args.Length -gt 0 -and -not [string]::IsNullOrWhiteSpace($args[0])) { $args[0] } elseif ($env:RELEASE_VERSION) { $env:RELEASE_VERSION } else { "v$version" }
$target = if ($env:TARGET) { $env:TARGET } else { 'x86_64-pc-windows-msvc' }
$distDir = if ($env:DIST_DIR) { $env:DIST_DIR } else { Join-Path $root 'dist' }
$artifactDir = Join-Path $distDir $releaseVersion

New-Item -ItemType Directory -Force -Path $artifactDir | Out-Null
$env:PLUMB_BUILD_VERSION = $releaseVersion
cargo build --release --locked -p plumb-cli --target $target

$archive = "$name-$target.zip"
$tmpdir = Join-Path ([System.IO.Path]::GetTempPath()) ("plumb-" + [System.Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmpdir | Out-Null
try {
    Copy-Item (Join-Path $root "target/$target/release/plumb.exe") (Join-Path $tmpdir 'plumb.exe')
    Compress-Archive -LiteralPath (Join-Path $tmpdir 'plumb.exe') -DestinationPath (Join-Path $artifactDir $archive) -Force
    Write-Output (Join-Path $artifactDir $archive)
}
finally {
    Remove-Item -LiteralPath $tmpdir -Recurse -Force -ErrorAction SilentlyContinue
}
