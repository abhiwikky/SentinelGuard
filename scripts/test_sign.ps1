#Requires -RunAsAdministrator
<#
.SYNOPSIS
    Creates a self-signed test certificate, signs the SentinelGuard driver,
    and enables Windows test signing mode.

.DESCRIPTION
    For development/testing only. NOT for production deployment.
    Requires admin privileges. Will reboot to apply test signing.

.PARAMETER DriverPath
    Path to sentinelguard.sys (default: driver\build\Release\x64\sentinelguard.sys)

.PARAMETER SkipReboot
    If set, skips the reboot prompt after enabling test signing.
#>

param(
    [string]$DriverPath = "$PSScriptRoot\..\driver\build\Release\x64\sentinelguard.sys",
    [switch]$SkipReboot
)

$ErrorActionPreference = "Stop"

function Write-Step($msg) {
    Write-Host "[SIGN] " -ForegroundColor Magenta -NoNewline
    Write-Host $msg
}

# ─── Check Admin ──────────────────────────────────────────────────────

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host "[ERROR] Run this script as Administrator." -ForegroundColor Red
    exit 1
}

# ─── Verify Driver Exists ─────────────────────────────────────────────

$DriverPath = (Resolve-Path $DriverPath -ErrorAction SilentlyContinue).Path
if (-not $DriverPath -or -not (Test-Path $DriverPath)) {
    Write-Host "[ERROR] Driver not found. Build it first with driver\build.bat" -ForegroundColor Red
    exit 1
}
Write-Step "Driver found: $DriverPath"

# ─── Create Self-Signed Test Certificate ──────────────────────────────

$certName = "SentinelGuard Test Driver Signing"
$existingCert = Get-ChildItem -Path "Cert:\LocalMachine\My" | Where-Object { $_.Subject -like "*$certName*" }

if ($existingCert) {
    Write-Step "Using existing test certificate: $($existingCert.Thumbprint)"
    $cert = $existingCert
} else {
    Write-Step "Creating self-signed test certificate..."
    $cert = New-SelfSignedCertificate `
        -Type CodeSigningCert `
        -Subject "CN=$certName" `
        -CertStoreLocation "Cert:\LocalMachine\My" `
        -NotAfter (Get-Date).AddYears(5) `
        -KeyUsage DigitalSignature `
        -KeyAlgorithm RSA `
        -KeyLength 2048 `
        -HashAlgorithm SHA256

    Write-Step "Certificate created: $($cert.Thumbprint)"

    # Also add to Trusted Root CA and Trusted Publishers
    $store = New-Object System.Security.Cryptography.X509Certificates.X509Store("Root", "LocalMachine")
    $store.Open("ReadWrite")
    $store.Add($cert)
    $store.Close()
    Write-Step "Added to Trusted Root CAs"

    $store = New-Object System.Security.Cryptography.X509Certificates.X509Store("TrustedPublisher", "LocalMachine")
    $store.Open("ReadWrite")
    $store.Add($cert)
    $store.Close()
    Write-Step "Added to Trusted Publishers"
}

# ─── Sign the Driver ──────────────────────────────────────────────────

Write-Step "Signing driver..."

# Find signtool.exe from Windows SDK
$signTool = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin" -Recurse -Filter "signtool.exe" |
    Where-Object { $_.FullName -match "x64" } |
    Sort-Object { $_.FullName } -Descending |
    Select-Object -First 1 -ExpandProperty FullName

if (-not $signTool) {
    Write-Host "[ERROR] signtool.exe not found. Ensure Windows SDK is installed." -ForegroundColor Red
    exit 1
}

Write-Step "Using signtool: $signTool"

# Sign with the test certificate
& $signTool sign /v /sm /s My /n "$certName" /t "http://timestamp.digicert.com" /fd SHA256 "$DriverPath"

if ($LASTEXITCODE -ne 0) {
    Write-Host "[WARNING] Signtool returned non-zero. Trying without timestamp..." -ForegroundColor Yellow
    & $signTool sign /v /sm /s My /n "$certName" /fd SHA256 "$DriverPath"
}

if ($LASTEXITCODE -eq 0) {
    Write-Step "Driver signed successfully!"
} else {
    Write-Host "[ERROR] Failed to sign driver." -ForegroundColor Red
    exit 1
}

# ─── Enable Test Signing ──────────────────────────────────────────────

Write-Step "Enabling test signing mode..."
$result = & bcdedit /set testsigning on 2>&1
Write-Step "bcdedit result: $result"

# ─── Summary ──────────────────────────────────────────────────────────

Write-Host ""
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Green
Write-Host "  Driver Signing Complete                          " -ForegroundColor Green
Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Green
Write-Host ""
Write-Host "  Certificate: $certName"
Write-Host "  Thumbprint:  $($cert.Thumbprint)"
Write-Host "  Driver:      $DriverPath"
Write-Host "  Test Signing: ENABLED"
Write-Host ""

if (-not $SkipReboot) {
    Write-Host "  A REBOOT is required for test signing to take effect." -ForegroundColor Yellow
    $response = Read-Host "  Reboot now? (y/N)"
    if ($response -eq 'y' -or $response -eq 'Y') {
        Restart-Computer -Force
    } else {
        Write-Host "  Remember to reboot before loading the driver." -ForegroundColor Yellow
    }
} else {
    Write-Host "  Remember to reboot for test signing to take effect." -ForegroundColor Yellow
}
