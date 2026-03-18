#Requires -RunAsAdministrator
<#
.SYNOPSIS
    SentinelGuard Development Build Script

.DESCRIPTION
    Safely stops the running agent/bridge, recompiles the Rust agent,
    deploys the new binary, and restarts everything.
#>

$ErrorActionPreference = "Stop"
$InstallDir = "C:\Program Files\SentinelGuard"
$DataDir = "C:\ProgramData\SentinelGuard"

Write-Host "[BUILD] Stopping active SentinelGuard processes..." -ForegroundColor Cyan

# Stop agent and bridge
Get-Process | Where-Object { $_.Name -match "^sentinelguard_agent$" } | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

Write-Host "[BUILD] Compiling Rust agent..." -ForegroundColor Cyan

Push-Location ".\agent"
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo build failed with exit code $LASTEXITCODE"
    }

    Write-Host "[BUILD] Deploying..." -ForegroundColor Cyan
    Copy-Item ".\target\release\sentinelguard_agent.exe" "..\artifacts\" -Force
    Copy-Item ".\target\release\sentinelguard_agent.exe" "$InstallDir\" -Force
} finally {
    Pop-Location
}

Write-Host "[BUILD] Restarting SentinelGuard..." -ForegroundColor Cyan

# Ensure driver is loaded
fltmc load sentinelguard 2>$null

# Start agent as background process
Start-Process -FilePath "$InstallDir\sentinelguard_agent.exe" `
    -ArgumentList "`"$DataDir\config.toml`"" `
    -WindowStyle Hidden `
    -RedirectStandardOutput "$DataDir\logs\agent_stdout.log" `
    -RedirectStandardError "$DataDir\logs\agent_stderr.log"

Start-Sleep -Seconds 2

$agentProc = Get-Process -Name "sentinelguard_agent" -ErrorAction SilentlyContinue
if ($agentProc) {
    Write-Host "[BUILD] Success! Agent running (PID: $($agentProc.Id))" -ForegroundColor Green
} else {
    Write-Host "[BUILD] WARNING: Agent may have failed to start. Check $DataDir\logs\" -ForegroundColor Yellow
}
