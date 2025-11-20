# SentinelGuard Uninstallation Script
# Requires Administrator privileges

$ErrorActionPreference = "Stop"

Write-Host "SentinelGuard Uninstallation Script" -ForegroundColor Red
Write-Host "====================================" -ForegroundColor Red

# Check for administrator privileges
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "ERROR: This script requires Administrator privileges." -ForegroundColor Red
    exit 1
}

# Stop and remove services
$services = @("SentinelGuardAgent", "SentinelGuard")

foreach ($serviceName in $services) {
    $service = Get-Service -Name $serviceName -ErrorAction SilentlyContinue
    if ($service) {
        Write-Host "Stopping service: $serviceName" -ForegroundColor Yellow
        Stop-Service -Name $serviceName -Force -ErrorAction SilentlyContinue
        
        Write-Host "Removing service: $serviceName" -ForegroundColor Yellow
        sc.exe delete $serviceName | Out-Null
    }
}

# Remove installation directory
$installPath = "C:\Program Files\SentinelGuard"
if (Test-Path $installPath) {
    Write-Host "Removing installation directory: $installPath" -ForegroundColor Yellow
    Remove-Item -Path $installPath -Recurse -Force -ErrorAction SilentlyContinue
}

# Remove ProgramData directory (optional - contains database)
$programDataPath = "$env:ProgramData\SentinelGuard"
$removeData = Read-Host "Remove database and logs? (y/N)"
if ($removeData -eq "y" -or $removeData -eq "Y") {
    Write-Host "Removing data directory: $programDataPath" -ForegroundColor Yellow
    Remove-Item -Path $programDataPath -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "Uninstallation completed!" -ForegroundColor Green

