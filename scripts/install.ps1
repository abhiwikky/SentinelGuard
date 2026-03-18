#Requires -RunAsAdministrator
param(
    [string]$InstallDir = "C:\Program Files\SentinelGuard",
    [string]$DataDir = "C:\ProgramData\SentinelGuard",
    [string]$ArtifactDir = ".\artifacts"
)

$ErrorActionPreference = "Stop"

function Write-Step($msg)  { Write-Host "[INSTALL] " -ForegroundColor Cyan   -NoNewline; Write-Host $msg }
function Write-Warn($msg)  { Write-Host "[WARNING] " -ForegroundColor Yellow -NoNewline; Write-Host $msg }
function Write-Err($msg)   { Write-Host "[ERROR]   " -ForegroundColor Red    -NoNewline; Write-Host $msg }
function Write-Ok($msg)    { Write-Host "[  OK   ] " -ForegroundColor Green  -NoNewline; Write-Host $msg }

function Confirm-Artifact($path, $description) {
    if (-not (Test-Path $path)) {
        Write-Err "Missing artifact: $description ($path)"
        return $false
    }
    $hash = (Get-FileHash $path -Algorithm SHA256).Hash
    Write-Step "  Verified: $description (SHA256: $($hash.Substring(0, 16))...)"
    return $true
}

# =====================================================================
#  PHASE 1: PRE-FLIGHT CHECKS
# =====================================================================

Write-Host ""
Write-Host "=======================================================" -ForegroundColor Cyan
Write-Host "  SentinelGuard Installer                               " -ForegroundColor Cyan
Write-Host "=======================================================" -ForegroundColor Cyan
Write-Host ""

Write-Step "Phase 1: Pre-flight checks"

# Admin check
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Err "This script MUST be run as Administrator."
    exit 1
}
Write-Ok "Running as Administrator"

# Node.js check
$nodeVersion = $null
try { $nodeVersion = (& node --version 2>$null) } catch {}
if (-not $nodeVersion) {
    Write-Err "Node.js is not installed or not in PATH."
    Write-Err "Install from: https://nodejs.org"
    exit 1
}
Write-Ok "Node.js detected: $nodeVersion"

# UI build check
$uiDistPath = ".\ui\dist"
if (-not (Test-Path "$uiDistPath\index.html")) {
    Write-Err "UI build not found at $uiDistPath\index.html"
    Write-Err "Run these commands first:"
    Write-Host "    cd ui && npm install && npm run build && cd .." -ForegroundColor White
    exit 1
}
Write-Ok "UI build found"

# Artifact directory check
if (-not (Test-Path $ArtifactDir)) {
    Write-Err "Artifact directory not found: $ArtifactDir"
    exit 1
}

Write-Step "Validating artifacts..."
$requiredArtifacts = @{
    "$ArtifactDir\sentinelguard_agent.exe" = "Rust agent binary"
    "$ArtifactDir\quarantine_helper.exe"   = "Quarantine helper binary"
    "$ArtifactDir\config.toml"             = "Configuration file"
    "$ArtifactDir\onnxruntime.dll"         = "ONNX Runtime DLL"
}

$allValid = $true
foreach ($kv in $requiredArtifacts.GetEnumerator()) {
    if (-not (Confirm-Artifact $kv.Key $kv.Value)) { $allValid = $false }
}

if (Test-Path "$ArtifactDir\sentinelguard.sys") {
    Confirm-Artifact "$ArtifactDir\sentinelguard.sys" "Kernel driver" | Out-Null
} else {
    Write-Warn "Optional: Kernel driver not found"
}

if (Test-Path "$ArtifactDir\model.onnx") {
    Confirm-Artifact "$ArtifactDir\model.onnx" "ONNX ML model" | Out-Null
} else {
    Write-Warn "Optional: ML model not found"
}

if (-not $allValid) {
    Write-Err "Required artifacts missing. Aborting."
    exit 1
}
Write-Ok "All required artifacts present"

# =====================================================================
#  PHASE 2: TEARDOWN
# =====================================================================

Write-Host ""
Write-Step "Phase 2: Tearing down existing installation"

# 2a. Kill agent
$agentProcs = Get-Process -Name "sentinelguard_agent" -ErrorAction SilentlyContinue
if ($agentProcs) {
    Write-Step "  Killing running agent (PID: $($agentProcs.Id -join ', '))..."
    $agentProcs | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
    Write-Ok "  Agent killed"
} else {
    Write-Step "  No running agent found"
}

# 2b. Kill node
$nodeProcs = Get-Process -Name "node" -ErrorAction SilentlyContinue
if ($nodeProcs) {
    Write-Step "  Killing Node.js processes..."
    $nodeProcs | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
    Write-Ok "  Node processes killed"
} else {
    Write-Step "  No running Node processes found"
}

# 2c. Unload driver
Write-Step "  Unloading kernel driver..."
fltmc unload sentinelguard 2>$null
if ($LASTEXITCODE -eq 0) {
    Write-Ok "  Driver unloaded"
    Start-Sleep -Seconds 1
} else {
    Write-Step "  Driver was not loaded (nothing to unload)"
}

# 2d. Delete stale services
$svc = Get-Service -Name "SentinelGuardAgent" -ErrorAction SilentlyContinue
if ($svc) {
    Write-Step "  Removing stale agent service..."
    & sc.exe delete "SentinelGuardAgent" 2>$null | Out-Null
    Start-Sleep -Seconds 1
}

$drvSvc = Get-Service -Name "sentinelguard" -ErrorAction SilentlyContinue
if ($drvSvc) {
    Write-Step "  Removing stale driver service..."
    & sc.exe delete "sentinelguard" 2>$null | Out-Null
    Start-Sleep -Seconds 1
}

# 2e. Free port 3001
$portInUse = Get-NetTCPConnection -LocalPort 3001 -ErrorAction SilentlyContinue
if ($portInUse) {
    $blockingPid = $portInUse[0].OwningProcess
    Write-Warn "  Port 3001 in use (PID: $blockingPid). Killing..."
    Stop-Process -Id $blockingPid -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
}

# 2f. Free port 50051
$grpcInUse = Get-NetTCPConnection -LocalPort 50051 -ErrorAction SilentlyContinue
if ($grpcInUse) {
    $blockingPid = $grpcInUse[0].OwningProcess
    Write-Warn "  Port 50051 in use (PID: $blockingPid). Killing..."
    Stop-Process -Id $blockingPid -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
}

# 2g. Wipe old install directory
if (Test-Path $InstallDir) {
    Write-Step "  Removing old install: $InstallDir"
    Remove-Item $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
    Write-Ok "  Old installation removed"
}

# 2h. Remove stale database
$dbPath = "$DataDir\sentinelguard.db"
if (Test-Path $dbPath) {
    Write-Step "  Removing stale database"
    Remove-Item $dbPath -Force -ErrorAction SilentlyContinue
    Remove-Item "$dbPath-wal" -Force -ErrorAction SilentlyContinue
    Remove-Item "$dbPath-shm" -Force -ErrorAction SilentlyContinue
    Write-Ok "  Database cleared"
}

Write-Ok "Teardown complete"

# =====================================================================
#  PHASE 3: DEPLOY FILES
# =====================================================================

Write-Host ""
Write-Step "Phase 3: Deploying files"

# Create directories
$dirs = @($InstallDir, $DataDir, "$DataDir\logs", "$InstallDir\proto", "$InstallDir\bridge", "$InstallDir\ui")
foreach ($dir in $dirs) {
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
}

# Binaries
Copy-Item "$ArtifactDir\sentinelguard_agent.exe" "$InstallDir\" -Force
Write-Step "  Agent       -> $InstallDir\"

Copy-Item "$ArtifactDir\quarantine_helper.exe" "$InstallDir\" -Force
Write-Step "  Quarantine  -> $InstallDir\"

Copy-Item "$ArtifactDir\onnxruntime.dll" "$InstallDir\" -Force
Write-Step "  ONNX DLL    -> $InstallDir\"

# Config (preserve existing)
if (-not (Test-Path "$DataDir\config.toml")) {
    Copy-Item "$ArtifactDir\config.toml" "$DataDir\" -Force
    Write-Step "  Config      -> $DataDir\"
} else {
    Write-Warn "  Config exists, not overwriting"
}

# ML Model
if (Test-Path "$ArtifactDir\model.onnx") {
    Copy-Item "$ArtifactDir\model.onnx" "$DataDir\" -Force
    Write-Step "  ML Model    -> $DataDir\"
}

# Proto
if (Test-Path "$ArtifactDir\proto") {
    Copy-Item "$ArtifactDir\proto\*" "$InstallDir\proto\" -Force -Recurse
    Write-Step "  Proto       -> $InstallDir\proto\"
}

# Bridge
if (Test-Path "$ArtifactDir\bridge") {
    Copy-Item "$ArtifactDir\bridge\*" "$InstallDir\bridge\" -Force -Recurse
    Write-Step "  Bridge      -> $InstallDir\bridge\"
}

# UI
Copy-Item "$uiDistPath\*" "$InstallDir\ui\" -Force -Recurse
Write-Step "  UI          -> $InstallDir\ui\"

Write-Ok "All files deployed"

# =====================================================================
#  PHASE 4: KERNEL DRIVER
# =====================================================================

Write-Host ""
Write-Step "Phase 4: Kernel driver"

if (Test-Path "$ArtifactDir\sentinelguard.sys") {
    Copy-Item "$ArtifactDir\sentinelguard.sys" "$env:windir\System32\drivers\" -Force
    Write-Step "  Driver -> $env:windir\System32\drivers\"

    try {
        & sc.exe create "sentinelguard" type= filesys binPath= "\SystemRoot\System32\drivers\sentinelguard.sys" start= demand DisplayName= "SentinelGuard Driver" 2>$null | Out-Null

        $regPath = "HKLM:\System\CurrentControlSet\Services\sentinelguard\Instances"
        New-Item -Path $regPath -Force | Out-Null
        New-ItemProperty -Path $regPath -Name "DefaultInstance" -Value "SentinelGuard Instance" -PropertyType String -Force | Out-Null

        $instPath = "$regPath\SentinelGuard Instance"
        New-Item -Path $instPath -Force | Out-Null
        New-ItemProperty -Path $instPath -Name "Altitude" -Value "370050" -PropertyType String -Force | Out-Null
        New-ItemProperty -Path $instPath -Name "Flags" -Value 0 -PropertyType DWord -Force | Out-Null

        Write-Ok "  Driver registered"
    } catch {
        Write-Warn "  Driver registration error: $_"
    }

    fltmc load sentinelguard 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Ok "  Driver loaded into kernel"
    } else {
        Write-Warn "  Driver load code $LASTEXITCODE (may need test signing)"
    }
} else {
    Write-Warn "No kernel driver in artifacts. Agent runs without kernel visibility."
}

# =====================================================================
#  PHASE 5: LAUNCH AGENT & BRIDGE
# =====================================================================

Write-Host ""
Write-Step "Phase 5: Starting services"

$agentExe = "$InstallDir\sentinelguard_agent.exe"
$configFile = "$DataDir\config.toml"

# 5a. Start agent
Write-Step "  Starting agent..."
$agentArgs = "`"$configFile`""
Start-Process -FilePath $agentExe -ArgumentList $agentArgs -WindowStyle Hidden -RedirectStandardOutput "$DataDir\logs\agent_stdout.log" -RedirectStandardError "$DataDir\logs\agent_stderr.log"

Start-Sleep -Seconds 3

$proc = Get-Process -Name "sentinelguard_agent" -ErrorAction SilentlyContinue
if ($proc) {
    Write-Ok "  Agent running (PID: $($proc.Id))"
} else {
    Write-Err "  Agent failed to start! Check: $DataDir\logs\agent_stderr.log"
}

# 5b. Wait for gRPC
$grpcReady = $false
for ($i = 0; $i -lt 5; $i++) {
    $conn = Get-NetTCPConnection -LocalPort 50051 -State Listen -ErrorAction SilentlyContinue
    if ($conn) { $grpcReady = $true; break }
    Start-Sleep -Seconds 1
}

if ($grpcReady) {
    Write-Ok "  gRPC listening on port 50051"
} else {
    Write-Warn "  gRPC not yet ready - bridge may retry"
}

# 5c. Start bridge
$bridgeDir = "$InstallDir\bridge"
if (Test-Path "$bridgeDir\server.js") {
    Write-Step "  Starting Node.js bridge..."
    Start-Process -FilePath "node" -ArgumentList "`"$bridgeDir\server.js`"" -WorkingDirectory $bridgeDir -WindowStyle Hidden -RedirectStandardOutput "$DataDir\logs\bridge_stdout.log" -RedirectStandardError "$DataDir\logs\bridge_stderr.log"

    Start-Sleep -Seconds 4

    $bridgeConn = Get-NetTCPConnection -LocalPort 3001 -State Listen -ErrorAction SilentlyContinue
    if ($bridgeConn) {
        Write-Ok "  Bridge listening on http://127.0.0.1:3001"
    } else {
        Write-Warn "  Bridge may not have started. Check: $DataDir\logs\bridge_stderr.log"
    }
} else {
    Write-Err "  Bridge server.js not found at $bridgeDir"
}

# =====================================================================
#  SUMMARY
# =====================================================================

Write-Host ""
Write-Host "=======================================================" -ForegroundColor Green
Write-Host "  Installation Complete                                 " -ForegroundColor Green
Write-Host "=======================================================" -ForegroundColor Green
Write-Host ""
Write-Host "  Install dir : $InstallDir"
Write-Host "  Data dir    : $DataDir"
Write-Host "  Logs        : $DataDir\logs\"
Write-Host ""

$aStatus = if (Get-Process -Name "sentinelguard_agent" -ErrorAction SilentlyContinue) { "RUNNING" } else { "STOPPED" }
$bStatus = if (Get-NetTCPConnection -LocalPort 3001 -State Listen -ErrorAction SilentlyContinue) { "RUNNING" } else { "STOPPED" }
$dLoaded = fltmc 2>$null | Select-String "sentinelguard"
$dStatus = if ($dLoaded) { "LOADED" } else { "NOT LOADED" }

$ac = if ($aStatus -eq "RUNNING") { "Green" } else { "Red" }
$bc = if ($bStatus -eq "RUNNING") { "Green" } else { "Red" }
$dc = if ($dStatus -eq "LOADED") { "Green" } else { "Yellow" }

Write-Host "  Component        Status"
Write-Host "  ---------        ------"
Write-Host "  Kernel Driver    " -NoNewline; Write-Host $dStatus -ForegroundColor $dc
Write-Host "  Agent            " -NoNewline; Write-Host $aStatus -ForegroundColor $ac
Write-Host "  Web Bridge       " -NoNewline; Write-Host $bStatus -ForegroundColor $bc
Write-Host ""

if ($aStatus -eq "RUNNING" -and $bStatus -eq "RUNNING") {
    Write-Host "  Dashboard: " -NoNewline
    Write-Host "http://127.0.0.1:3001" -ForegroundColor Cyan
} else {
    Write-Host "  Some components failed. Check: $DataDir\logs\" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "  To stop everything:" -ForegroundColor DarkGray
Write-Host "    Stop-Process -Name sentinelguard_agent -Force" -ForegroundColor DarkGray
Write-Host "    Stop-Process -Name node -Force" -ForegroundColor DarkGray
Write-Host "    fltmc unload sentinelguard" -ForegroundColor DarkGray
Write-Host ""
