#Requires -RunAsAdministrator
<#
.SYNOPSIS
    SentinelGuard Uninstaller

.DESCRIPTION
    Removes the SentinelGuard ransomware detection platform.
    Must be run as Administrator.
    Preserves database and logs by default.

.PARAMETER RemoveData
    If set, also removes configuration, database, and logs.

.PARAMETER InstallDir
    Installation directory (default: C:\Program Files\SentinelGuard)

.PARAMETER DataDir
    Data directory (default: C:\ProgramData\SentinelGuard)
#>

param(
    [switch]$RemoveData,
    [string]$InstallDir = "C:\Program Files\SentinelGuard",
    [string]$DataDir = "C:\ProgramData\SentinelGuard"
)

$ErrorActionPreference = "Stop"

function Write-Step($msg) {
    Write-Host "[UNINSTALL] " -ForegroundColor Yellow -NoNewline
    Write-Host $msg
}

# ─── Check Admin ──────────────────────────────────────────────────────

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host "[ERROR] This script must be run as Administrator." -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Yellow
Write-Host "  SentinelGuard Uninstaller                        " -ForegroundColor Yellow
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Yellow
Write-Host ""

# ─── Stop and Remove Service ─────────────────────────────────────────

$serviceName = "SentinelGuardAgent"
$service = Get-Service -Name $serviceName -ErrorAction SilentlyContinue

if ($service) {
    Write-Step "Stopping service..."
    Stop-Service -Name $serviceName -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2

    Write-Step "Removing service..."
    & sc.exe delete $serviceName | Out-Null
    Write-Step "  Service removed"
} else {
    Write-Step "Service not found, skipping"
}

# ─── Unload and Remove Driver ────────────────────────────────────────

Write-Step "Checking for kernel driver..."

try {
    $filter = & fltmc filters 2>&1 | Select-String "SentinelGuard"
    if ($filter) {
        Write-Step "Unloading minifilter..."
        & fltmc unload SentinelGuard 2>&1 | Out-Null
        Start-Sleep -Seconds 1
    }
} catch {
    Write-Step "  Filter manager check skipped"
}

$driverPath = "$env:windir\System32\drivers\sentinelguard.sys"
if (Test-Path $driverPath) {
    Remove-Item $driverPath -Force -ErrorAction SilentlyContinue
    Write-Step "  Removed: $driverPath"
}

# ─── Remove Installation Files ───────────────────────────────────────

if (Test-Path $InstallDir) {
    Write-Step "Removing installation directory..."
    Remove-Item $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
    Write-Step "  Removed: $InstallDir"
} else {
    Write-Step "Installation directory not found, skipping"
}

# ─── Remove Data (Optional) ──────────────────────────────────────────

if ($RemoveData) {
    if (Test-Path $DataDir) {
        Write-Step "Removing data directory (database, config, logs)..."
        Remove-Item $DataDir -Recurse -Force -ErrorAction SilentlyContinue
        Write-Step "  Removed: $DataDir"
    }
} else {
    Write-Step "Data directory preserved at $DataDir"
    Write-Step "  (use -RemoveData flag to delete config, database, and logs)"
}

# ─── Clean Up Registry ───────────────────────────────────────────────

$regPath = "HKLM:\SYSTEM\CurrentControlSet\Services\SentinelGuard"
if (Test-Path $regPath) {
    Remove-Item $regPath -Recurse -Force -ErrorAction SilentlyContinue
    Write-Step "  Cleaned up registry entries"
}

# ─── Done ─────────────────────────────────────────────────────────────

Write-Host ""
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Green
Write-Host "  Uninstall Complete                                " -ForegroundColor Green
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Green
Write-Host ""
