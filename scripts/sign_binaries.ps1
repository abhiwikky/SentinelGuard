# Code Signing Script for SentinelGuard Binaries
# Requires a valid code signing certificate

param(
    [string]$CertificatePath = "",
    [string]$CertificatePassword = "",
    [string]$TimestampServer = "http://timestamp.digicert.com"
)

$ErrorActionPreference = "Stop"

Write-Host "SentinelGuard Code Signing Script" -ForegroundColor Green
Write-Host "=================================" -ForegroundColor Green

if (-not $CertificatePath) {
    Write-Host "ERROR: Certificate path required" -ForegroundColor Red
    Write-Host "Usage: .\sign_binaries.ps1 -CertificatePath <path> -CertificatePassword <password>" -ForegroundColor Yellow
    exit 1
}

$signtool = Get-Command signtool -ErrorAction SilentlyContinue
if (-not $signtool) {
    Write-Host "ERROR: signtool not found. Install Windows SDK." -ForegroundColor Red
    exit 1
}

$binaries = @(
    "agent\target\release\sentinelguard-agent.exe",
    "quarantine\build\Release\quarantine.exe",
    "kernel\build\Release\SentinelGuard.sys"
)

foreach ($binary in $binaries) {
    if (Test-Path $binary) {
        Write-Host "Signing: $binary" -ForegroundColor Yellow

        $signArgs = @(
            "sign",
            "/f", $CertificatePath,
            "/p", $CertificatePassword,
            "/t", $TimestampServer,
            "/v",
            $binary
        )

        & signtool @signArgs

        if ($LASTEXITCODE -eq 0) {
            Write-Host "  [OK] Signed successfully" -ForegroundColor Green
            & signtool verify /pa /v $binary
        } else {
            Write-Host "  [FAIL] Signing failed" -ForegroundColor Red
        }
    } else {
        Write-Host "  [WARN] File not found: $binary" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "Code signing completed!" -ForegroundColor Green
