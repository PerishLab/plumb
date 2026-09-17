param(
  [Parameter(Mandatory = $true, Position = 0)]
  [string]$Marker,
  [Parameter(Position = 1)]
  [string]$Remote = "origin"
)

$ErrorActionPreference = "Stop"
$reference = "refs/tags/$Marker"
$references = [System.Collections.Generic.List[string]]::new()
$references.Add("+${reference}:${reference}")
if (-not $Marker.Contains("-")) {
  $references.Add("+refs/tags/${Marker}-*:refs/tags/${Marker}-*")
}

$arguments = [System.Collections.Generic.List[string]]::new()
if ($env:PLUMB_CHECKOUT_TOKEN) {
  $bytes = [System.Text.Encoding]::UTF8.GetBytes("x-access-token:$($env:PLUMB_CHECKOUT_TOKEN)")
  $basic = [Convert]::ToBase64String($bytes)
  $arguments.Add("-c")
  $arguments.Add("http.extraheader=AUTHORIZATION: basic $basic")
}
$arguments.Add("fetch")
$arguments.Add("--no-tags")
$arguments.Add("--depth=1")
$arguments.Add($Remote)
foreach ($held in $references) {
  $arguments.Add($held)
}

for ($attempt = 1; $attempt -le 3; $attempt++) {
  $start = [System.Diagnostics.ProcessStartInfo]::new()
  $start.FileName = "git"
  $start.UseShellExecute = $false
  foreach ($argument in $arguments) {
    [void]$start.ArgumentList.Add($argument)
  }
  $process = [System.Diagnostics.Process]::Start($start)
  if (-not $process.WaitForExit(45000)) {
    $process.Kill($true)
    $process.WaitForExit()
  }
  if ($process.ExitCode -eq 0) {
    git rev-parse --verify "$reference^{commit}" *> $null
    if ($LASTEXITCODE -eq 0) {
      exit 0
    }
  }
  Start-Sleep -Seconds $attempt
}

throw "cannot fetch release marker $Marker"
