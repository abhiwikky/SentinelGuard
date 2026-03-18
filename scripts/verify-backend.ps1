#Requires -RunAsAdministrator

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectDir = Split-Path -Parent $scriptDir
$nodeScript = "$projectDir\bridge\verify-backend.js"

if (Test-Path $nodeScript) {
    node $nodeScript
} else {
    Write-Host "Ciritcal Error: verify-backend.js not found in local bridge directory." -ForegroundColor Red
    
    # Try the deployed bridge folder
    $deployedScript = "C:\Program Files\SentinelGuard\bridge\verify-backend.js"
    if (Test-Path $deployedScript) {
        node $deployedScript
    } else {
        Write-Host "Could not locate backend verification script. Ensure SentinelGuard is built." -ForegroundColor Red
        exit 1
    }
}
