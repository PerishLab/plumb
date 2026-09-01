$ErrorActionPreference = "Stop"
& plumb ship execute --request $env:PLUMB_SHIP_REQUEST
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
