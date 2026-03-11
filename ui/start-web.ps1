param(
    [int]$Port = 4173
)

$env:SG_UI_PORT = "$Port"
node "$PSScriptRoot\server.js"
