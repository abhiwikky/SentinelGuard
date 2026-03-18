<#
.SYNOPSIS
    SentinelGuard Code Signing Workflow

.DESCRIPTION
    Signs the driver, agent, and quarantine helper binaries.
    Requires a valid code signing certificate.

.PARAMETER CertThumbprint
    Thumbprint of the code signing certificate in the local machine store.

.PARAMETER TimestampServer
    RFC 3161 timestamp server URL.

.PARAMETER ArtifactDir
    Directory containing built artifacts to sign.
#>

param(
    [Parameter(Mandatory=$true)]
    [string]$CertThumbprint,

    [string]$TimestampServer = "http://timestamp.digicert.com",
    [string]$ArtifactDir = ".\artifacts"
)

$ErrorActionPreference = "Stop"

function Write-Step($msg) {
    Write-Host "[SIGN] " -ForegroundColor Magenta -NoNewline
    Write-Host $msg
}

# ─── Validate Certificate ────────────────────────────────────────────

Write-Step "Looking for certificate with thumbprint: $CertThumbprint"

$cert = Get-ChildItem -Path "Cert:\LocalMachine\My" | Where-Object { $_.Thumbprint -eq $CertThumbprint }

if (-not $cert) {
    $cert = Get-ChildItem -Path "Cert:\CurrentUser\My" | Where-Object { $_.Thumbprint -eq $CertThumbprint }
}

if (-not $cert) {
    Write-Host "[ERROR] Certificate not found with thumbprint: $CertThumbprint" -ForegroundColor Red
    exit 1
}

Write-Step "Found certificate: $($cert.Subject)"

# ─── Sign Binaries ───────────────────────────────────────────────────

$filesToSign = @(
    @{ Path = "$ArtifactDir\sentinelguard_agent.exe"; Desc = "Rust Agent" },
    @{ Path = "$ArtifactDir\quarantine_helper.exe"; Desc = "Quarantine Helper" },
    @{ Path = "$ArtifactDir\sentinelguard.sys"; Desc = "Kernel Driver" }
)

foreach ($file in $filesToSign) {
    if (Test-Path $file.Path) {
        Write-Step "Signing $($file.Desc): $($file.Path)"

        try {
            $result = Set-AuthenticodeSignature `
                -FilePath $file.Path `
                -Certificate $cert `
                -TimestampServer $TimestampServer `
                -HashAlgorithm SHA256

            if ($result.Status -eq "Valid") {
                Write-Step "  Signed successfully"
            } else {
                Write-Host "[WARNING] Signature status: $($result.Status)" -ForegroundColor Yellow
                Write-Host "  Message: $($result.StatusMessage)" -ForegroundColor Yellow
            }
        } catch {
            Write-Host "[ERROR] Failed to sign $($file.Desc): $_" -ForegroundColor Red
        }
    } else {
        Write-Host "[SKIP] $($file.Desc) not found at $($file.Path)" -ForegroundColor Gray
    }
}

# ─── Create Catalog for Driver ────────────────────────────────────────

if (Test-Path "$ArtifactDir\sentinelguard.sys") {
    Write-Step "Note: For production driver signing, use the Windows Hardware Dev Center"
    Write-Step "or create a catalog file with Inf2Cat and sign it with SignTool."
    Write-Step "Test signing can be enabled with: bcdedit /set testsigning on"
}

Write-Host ""
Write-Step "Signing workflow complete."
