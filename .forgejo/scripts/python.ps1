param(
    [ValidateSet("probe", "install")]
    [string]$Phase = "probe"
)
$ErrorActionPreference = "Stop"
$contract = "windows-x64-python-3.13.15-v1"
$checksum = "6479223746cdfb79d25865110d6f524ac98de081324e119af1dc3ae36bddc7a5"
$address = "https://www.python.org/ftp/python/3.13.15/python-3.13.15-amd64.zip"
$relative = ".plumb-runtime/$contract/python.zip"
$cache = Join-Path $env:GITHUB_WORKSPACE ".plumb-runtime/$contract"
$archive = Join-Path $cache "python.zip"

function Confirm-Python([string]$Executable) {
    $probe = "import sys,json,pathlib,ssl,hashlib,subprocess,lzma,tarfile,zipfile; assert sys.version_info >= (3,9); print(json.dumps(sys.executable))"
    try {
        $result = & $Executable -I -c $probe 2>$null
        if ($LASTEXITCODE -eq 0) {
            return ($result | ConvertFrom-Json)
        }
    } catch {}
    return $null
}

function Publish-Python([string]$Executable) {
    if ($Executable.Contains("`n") -or $Executable.Contains("`r")) {
        throw "Python executable path cannot contain a newline"
    }
    "PLUMB_WORKFLOW_PYTHON=$Executable" | Out-File $env:GITHUB_ENV -Encoding utf8 -Append
    Write-Host "Control interpreter: $Executable"
}

if ($Phase -eq "probe") {
    foreach ($name in @("python.exe", "python3.exe")) {
        foreach ($command in @(Get-Command $name -CommandType Application -All -ErrorAction SilentlyContinue)) {
            if ($command.Source -match '[\\/]Microsoft[\\/]WindowsApps[\\/]') { continue }
            $executable = Confirm-Python $command.Source
            if ($executable) {
                Publish-Python $executable
                "needed=false" | Out-File $env:GITHUB_OUTPUT -Encoding utf8 -Append
                exit 0
            }
        }
    }
    if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne "X64") {
        throw "No compatible Python found and no bootstrap archive is declared for this Windows architecture"
    }
    @("needed=true", "archive=$relative", "key=$contract-$checksum") |
        Out-File $env:GITHUB_OUTPUT -Encoding utf8 -Append
    exit 0
}

New-Item -ItemType Directory -Force $cache | Out-Null
if (-not (Test-Path -LiteralPath $archive)) {
    $download = Join-Path $cache ("download-" + [guid]::NewGuid().ToString("N") + ".zip")
    Invoke-WebRequest -Uri $address -OutFile $download -TimeoutSec 120
    if ((Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash.ToLowerInvariant() -ne $checksum) {
        throw "Downloaded Python archive checksum mismatch; retained $download"
    }
    Move-Item -LiteralPath $download -Destination $archive
}
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $checksum) {
    throw "Cached Python archive checksum mismatch; retained $archive"
}
$destination = Join-Path $env:RUNNER_TEMP ("plumb-python-" + [guid]::NewGuid().ToString("N"))
Expand-Archive -LiteralPath $archive -DestinationPath $destination
$executable = Confirm-Python (Join-Path $destination "python.exe")
if (-not $executable) { throw "Isolated Python failed its execution probe; retained $destination" }
Publish-Python $executable
