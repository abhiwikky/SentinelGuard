param (
    [ValidateSet("MassWrite", "MassRename", "RansomNote", "ExtensionExplosion", "HighEntropy", "ProcessBehavior", "ShadowCopy", "All")]
    [string]$TestName = "All",
    [string]$TestDir = "C:\Temp\SentinelGuardTests"
)

$ErrorActionPreference = "Stop"

function Write-Info($msg) { Write-Host "[INFO] $msg" -ForegroundColor Cyan }
function Write-Success($msg) { Write-Host "[OK]   $msg" -ForegroundColor Green }

# Cleanup and prep
if (Test-Path $TestDir) { Remove-Item $TestDir -Recurse -Force -ErrorAction SilentlyContinue }
New-Item -ItemType Directory -Path $TestDir -Force | Out-Null
Set-Location $TestDir

# 1. Mass Write Test (Threshold: > 50 writes in 10s)
if ($TestName -eq "MassWrite" -or $TestName -eq "All") {
    Write-Info "Starting MassWrite test (creating 60 files quickly)..."
    for ($i = 0; $i -lt 60; $i++) {
        Set-Content -Path "mass_write_$i.txt" -Value "Dummy data for file $i"
    }
    Write-Success "MassWrite test complete."
    if ($TestName -eq "All") { Start-Sleep -Seconds 2 }
}

# 2. Mass Rename/Delete Test (Threshold: > 20 renames/deletes in 10s)
if ($TestName -eq "MassRename" -or $TestName -eq "All") {
    Write-Info "Starting MassRename/Delete test (creating, renaming 30 files)..."
    for ($i = 0; $i -lt 30; $i++) {
        Set-Content -Path "rename_test_$i.txt" -Value "Data"
    }
    for ($i = 0; $i -lt 30; $i++) {
        Rename-Item -Path "rename_test_$i.txt" -NewName "rename_test_$i.encrypted"
    }
    Write-Success "MassRename test complete."
    if ($TestName -eq "All") { Start-Sleep -Seconds 2 }
}

# 3. Ransom Note Test (Looks for readme.txt, decrypt_instructions, etc.)
if ($TestName -eq "RansomNote" -or $TestName -eq "All") {
    Write-Info "Starting RansomNote test (dropping decrypt_instructions.txt)..."
    Set-Content -Path "decrypt_instructions.txt" -Value "Your files have been encrypted! Pay 1 BTC."
    Set-Content -Path "readme.txt" -Value "Your files have been encrypted! Pay 1 BTC."
    Write-Success "RansomNote test complete."
    if ($TestName -eq "All") { Start-Sleep -Seconds 2 }
}

# 4. Extension Explosion Test (Threshold: > 10 unique extensions in 30s)
if ($TestName -eq "ExtensionExplosion" -or $TestName -eq "All") {
    Write-Info "Starting ExtensionExplosion test (writing 15 different extensions)..."
    $exts = @(".crypt", ".locked", ".enc", ".rnsm", ".bad", ".evil", ".zzzz", ".crypted", ".locky", ".crypto", ".pay", ".btc", ".wallet", ".dark", ".hacked")
    foreach ($ext in $exts) {
        Set-Content -Path "explodetest$ext" -Value "Data"
    }
    Write-Success "ExtensionExplosion test complete."
    if ($TestName -eq "All") { Start-Sleep -Seconds 2 }
}

# 5. High Entropy Test (Threshold: > 7.0 entropy on files >= 1024 bytes)
if ($TestName -eq "HighEntropy" -or $TestName -eq "All") {
    Write-Info "Starting HighEntropy test (writing 10 high-entropy files across directories)..."
    $rand = New-Object Security.Cryptography.RNGCryptoServiceProvider
    for ($i = 0; $i -lt 10; $i++) {
        $subdir = "$TestDir\entropy_dir_$i"
        New-Item -ItemType Directory -Path $subdir -Force | Out-Null
        $bytes = New-Object Byte[] 4096
        $rand.GetBytes($bytes)
        [System.IO.File]::WriteAllBytes("$subdir\encrypted_$i.dat", $bytes)
    }
    $rand.Dispose()
    Write-Success "HighEntropy test complete."
    if ($TestName -eq "All") { Start-Sleep -Seconds 2 }
}

# 6. Process Behavior Test (Threshold: > 15 extensions OR > 50 directories)
if ($TestName -eq "ProcessBehavior" -or $TestName -eq "All") {
    Write-Info "Starting ProcessBehavior test (writing to 55 different directories)..."
    for ($i = 0; $i -lt 55; $i++) {
        $subdir = "$TestDir\dir_$i"
        New-Item -ItemType Directory -Path $subdir | Out-Null
        Set-Content -Path "$subdir\behavior_test.txt" -Value "Data"
    }
    Write-Success "ProcessBehavior test complete."
    if ($TestName -eq "All") { Start-Sleep -Seconds 2 }
}

# 7. Shadow Copy Test
if ($TestName -eq "ShadowCopy" -or $TestName -eq "All") {
    Write-Info "Starting ShadowCopy test (simulating vssadmin launch)..."
    # We don't actually want to delete real shadow copies on the user's dev machine, 
    # so we just call it with invalid arguments to trigger the process name in the driver telemetry.
    try {
        & vssadmin.exe DummyFakeCommand ToTriggerTelemetry 2>$null
    } catch { }
    Write-Success "ShadowCopy test complete."
}

Write-Info "All selected tests finished running."
Write-Info "Check the SentinelGuard Dashboard for Process Risk scores!"
