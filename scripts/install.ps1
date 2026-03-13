# SentinelGuard Installation Script
# Requires Administrator privileges

param(
    [string]$InstallPath = "C:\Program Files\SentinelGuard",
    [switch]$SkipDriver = $false,
    [switch]$SkipAgentService = $false
)

$ErrorActionPreference = "Stop"

Write-Host "SentinelGuard Installation Script" -ForegroundColor Green
Write-Host "=================================" -ForegroundColor Green

$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Split-Path -Parent $ScriptRoot

function Get-SentinelGuardDriverPackages {
    $pnputilOutput = & pnputil.exe /enum-drivers 2>&1
    $packages = @()
    $current = @{}

    foreach ($line in $pnputilOutput) {
        if ($line -match '^\s*Published Name:\s*(\S+)') {
            if ($current.Count -gt 0) {
                $packages += [PSCustomObject]$current
                $current = @{}
            }
            $current.PublishedName = $matches[1].Trim()
        } elseif ($line -match '^\s*Original Name:\s*(.+)$') {
            $current.OriginalName = $matches[1].Trim()
        } elseif ($line -match '^\s*Provider Name:\s*(.+)$') {
            $current.ProviderName = $matches[1].Trim()
        } elseif ([string]::IsNullOrWhiteSpace($line) -and $current.Count -gt 0) {
            $packages += [PSCustomObject]$current
            $current = @{}
        }
    }

    if ($current.Count -gt 0) {
        $packages += [PSCustomObject]$current
    }

    return $packages | Where-Object {
        $_.OriginalName -ieq "sentinelguard.inf" -or $_.ProviderName -ieq "SentinelGuard"
    }
}

# Check for administrator privileges
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "ERROR: This script requires Administrator privileges." -ForegroundColor Red
    exit 1
}

# Create installation directory
Write-Host "Creating installation directory: $InstallPath" -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path $InstallPath | Out-Null
New-Item -ItemType Directory -Force -Path "$InstallPath\logs" | Out-Null
New-Item -ItemType Directory -Force -Path "$InstallPath\config" | Out-Null
New-Item -ItemType Directory -Force -Path "$InstallPath\models" | Out-Null
$UiInstallPath = Join-Path $InstallPath "ui"
if (Test-Path $UiInstallPath -PathType Leaf) {
    Write-Host "Removing conflicting UI file at: $UiInstallPath" -ForegroundColor Yellow
    Remove-Item -Force $UiInstallPath
}
New-Item -ItemType Directory -Force -Path $UiInstallPath | Out-Null
$UiDistInstallPath = Join-Path $UiInstallPath "dist"
New-Item -ItemType Directory -Force -Path $UiDistInstallPath | Out-Null

$AgentExe = Join-Path $RepoRoot "agent\target\release\sentinelguard-agent.exe"
$QuarantineExe = Join-Path $RepoRoot "quarantine\build\Release\quarantine.exe"
$UiDist = Join-Path $RepoRoot "ui\dist"
$UiServer = Join-Path $RepoRoot "ui\server.js"
$UiStartScript = Join-Path $RepoRoot "ui\start-web.ps1"
$UiPackageJson = Join-Path $RepoRoot "ui\package.json"
$UiNodeModules = Join-Path $RepoRoot "ui\node_modules"
$UiProtoSource = Join-Path $RepoRoot "agent\proto\sentinelguard.proto"
$ModelGlob = Join-Path $RepoRoot "ml\models\*.onnx"
$ConfigToml = Join-Path $RepoRoot "agent\config\config.toml"
$DriverSys = Join-Path $RepoRoot "kernel\build\Release\SentinelGuard.sys"
$DriverInf = Join-Path $RepoRoot "kernel\SentinelGuard.inf"
$DriverCat = Join-Path $RepoRoot "kernel\SentinelGuard.cat"

if (-not (Test-Path $AgentExe)) {
    throw "Missing agent binary: $AgentExe. Build it with: cd agent; cargo build --release"
}
if (-not (Test-Path $QuarantineExe)) {
    throw "Missing quarantine binary: $QuarantineExe. Build it from quarantine\build (Release)."
}
if (-not (Test-Path $UiDist)) {
    throw "Missing UI dist folder: $UiDist. Build it with: cd ui; npm run build:web"
}
if (-not (Test-Path $UiServer)) {
    throw "Missing UI web bridge: $UiServer"
}
if (-not (Test-Path $UiStartScript)) {
    throw "Missing UI start script: $UiStartScript"
}
if (-not (Test-Path $UiPackageJson)) {
    throw "Missing UI package.json: $UiPackageJson"
}
if (-not (Test-Path $UiNodeModules)) {
    throw "Missing UI node_modules: $UiNodeModules. Install UI dependencies with: cd ui; npm install"
}
if (-not (Test-Path $UiProtoSource)) {
    throw "Missing UI proto source: $UiProtoSource"
}
if (-not (Get-ChildItem -Path $ModelGlob -ErrorAction SilentlyContinue)) {
    throw "Missing ONNX model(s) at: $ModelGlob. Generate/copy model files before install."
}
if (-not (Test-Path $ConfigToml)) {
    throw "Missing config file: $ConfigToml"
}
if (-not (Test-Path $DriverInf)) {
    throw "Missing driver INF: $DriverInf"
}
if (-not $SkipDriver -and -not (Test-Path $DriverCat)) {
    throw "Missing driver catalog: $DriverCat. Generate/sign it before install."
}

# Copy agent executable
Write-Host "Installing agent..." -ForegroundColor Yellow
Copy-Item $AgentExe -Destination "$InstallPath\sentinelguard-agent.exe" -Force

# Copy quarantine module
Write-Host "Installing quarantine module..." -ForegroundColor Yellow
Copy-Item $QuarantineExe -Destination "$InstallPath\quarantine.exe" -Force

# Copy UI
Write-Host "Installing browser UI..." -ForegroundColor Yellow
Copy-Item (Join-Path $UiDist "*") -Destination $UiDistInstallPath -Recurse -Force
Copy-Item $UiServer -Destination (Join-Path $UiInstallPath "server.js") -Force
Copy-Item $UiStartScript -Destination (Join-Path $UiInstallPath "start-web.ps1") -Force
Copy-Item $UiPackageJson -Destination (Join-Path $UiInstallPath "package.json") -Force
Copy-Item $UiNodeModules -Destination (Join-Path $UiInstallPath "node_modules") -Recurse -Force
$UiProtoInstallPath = Join-Path $UiInstallPath "proto"
New-Item -ItemType Directory -Force -Path $UiProtoInstallPath | Out-Null
Copy-Item $UiProtoSource -Destination (Join-Path $UiProtoInstallPath "sentinelguard.proto") -Force

# Copy ML model
Write-Host "Installing ML model..." -ForegroundColor Yellow
Copy-Item $ModelGlob -Destination "$InstallPath\models\" -Force

# Copy configuration
Write-Host "Installing configuration..." -ForegroundColor Yellow
Copy-Item $ConfigToml -Destination "$InstallPath\config\config.toml" -Force

# Install kernel driver
if (-not $SkipDriver) {
    if (-not (Test-Path $DriverSys)) {
        throw "Missing kernel driver binary: $DriverSys. Build it from kernel\build (Release), or rerun with -SkipDriver."
    }
    Write-Host "Installing kernel driver..." -ForegroundColor Yellow
    $driverPath = "$InstallPath\SentinelGuard.sys"
    $driverInfPath = "$InstallPath\SentinelGuard.inf"
    $driverCatPath = "$InstallPath\SentinelGuard.cat"
    Copy-Item $DriverSys -Destination $driverPath -Force
    Copy-Item $DriverInf -Destination $driverInfPath -Force
    Copy-Item $DriverCat -Destination $driverCatPath -Force

    $serviceName = "SentinelGuard"
    $existingService = Get-Service -Name $serviceName -ErrorAction SilentlyContinue

    if ($existingService) {
        Write-Host "Stopping existing driver service..." -ForegroundColor Yellow
        & fltmc.exe unload $serviceName 2>$null | Out-Null
        Stop-Service -Name $serviceName -Force -ErrorAction SilentlyContinue
        sc.exe delete $serviceName | Out-Null
    }

    $existingDriverPackages = Get-SentinelGuardDriverPackages
    if ($existingDriverPackages) {
        Write-Host "Removing stale SentinelGuard driver packages..." -ForegroundColor Yellow
        foreach ($driverPackage in ($existingDriverPackages | Sort-Object PublishedName -Descending)) {
            Write-Host "Deleting $($driverPackage.PublishedName) from the driver store..." -ForegroundColor Yellow
            $deleteOutput = & pnputil.exe /delete-driver $driverPackage.PublishedName /uninstall /force 2>&1
            if ($LASTEXITCODE -ne 0) {
                Write-Host $deleteOutput
                throw "Failed to delete stale driver package $($driverPackage.PublishedName)."
            }
        }
    }

    Write-Host "Registering kernel minifilter with SetupAPI..." -ForegroundColor Yellow
    $signature = Get-AuthenticodeSignature $driverPath
    if ($signature.Status -eq "NotSigned") {
        throw "Kernel driver is not signed: $driverPath. Sign it before installation or boot Windows in testsigning mode."
    }
    if ($signature.Status -ne "Valid") {
        Write-Host "WARNING: Driver signature status is $($signature.Status). $($signature.StatusMessage)" -ForegroundColor Yellow
    }

    $pnputilOutput = & pnputil.exe /add-driver $driverInfPath /install 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host $pnputilOutput
        throw "Failed to register the kernel minifilter package with pnputil."
    }

    Write-Host "Creating kernel minifilter service via SetupAPI..." -ForegroundColor Yellow
    & rundll32.exe setupapi.dll,InstallHinfSection DefaultInstall.NTamd64 132 $driverInfPath
    Start-Sleep -Seconds 2

    $serviceRegPath = "HKLM:\SYSTEM\CurrentControlSet\Services\$serviceName"
    $instancesRegPath = Join-Path $serviceRegPath "Parameters\Instances"
    if (-not (Test-Path $serviceRegPath)) {
        Write-Host $pnputilOutput
        throw "Kernel minifilter registration did not create $serviceRegPath."
    }
    if (-not (Test-Path $instancesRegPath)) {
        Write-Host $pnputilOutput
        throw "Kernel minifilter registration did not create $instancesRegPath."
    }

    Write-Host "Kernel minifilter registered successfully" -ForegroundColor Green

    Write-Host "Loading kernel minifilter..." -ForegroundColor Yellow
    $loadOutput = & fltmc.exe load $serviceName 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Kernel minifilter loaded successfully" -ForegroundColor Green
    } else {
        Write-Host $loadOutput
        throw "Kernel minifilter registration succeeded, but filter load failed."
    }
}

$agentServiceName = "SentinelGuardAgent"
if (-not $SkipAgentService) {
    # Install agent as Windows service
    Write-Host "Installing agent service..." -ForegroundColor Yellow
    $existingAgentService = Get-Service -Name $agentServiceName -ErrorAction SilentlyContinue

    if ($existingAgentService) {
        Write-Host "Stopping existing agent service..." -ForegroundColor Yellow
        Stop-Service -Name $agentServiceName -Force -ErrorAction SilentlyContinue
        sc.exe delete $agentServiceName | Out-Null
    }

    Write-Host "Creating agent service..." -ForegroundColor Yellow
    sc.exe create $agentServiceName binPath= "$InstallPath\sentinelguard-agent.exe" start= auto DisplayName= "SentinelGuard Agent"

    if ($LASTEXITCODE -eq 0) {
        Write-Host "Agent service created successfully" -ForegroundColor Green
        try {
            Start-Service -Name $agentServiceName -ErrorAction Stop
            Write-Host "Agent service started" -ForegroundColor Green
        } catch {
            Write-Host "WARNING: Agent service could not be started." -ForegroundColor Red
            Write-Host "The current agent binary is likely not running as a native Windows service yet." -ForegroundColor Yellow
            Write-Host "You can run it manually for now: $InstallPath\sentinelguard-agent.exe" -ForegroundColor Yellow
        }
    } else {
        Write-Host "ERROR: Failed to create agent service" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "Skipping agent service installation (-SkipAgentService)" -ForegroundColor Yellow
}

# Create ProgramData directory for database
$programDataPath = "$env:ProgramData\SentinelGuard"
New-Item -ItemType Directory -Force -Path $programDataPath | Out-Null

# Set permissions
Write-Host "Setting permissions..." -ForegroundColor Yellow
$acl = Get-Acl $programDataPath
$permission = "NT AUTHORITY\SYSTEM","FullControl","ContainerInherit,ObjectInherit","None","Allow"
$accessRule = New-Object System.Security.AccessControl.FileSystemAccessRule $permission
$acl.SetAccessRule($accessRule)
Set-Acl $programDataPath $acl

Write-Host ""
Write-Host "Installation completed successfully!" -ForegroundColor Green
Write-Host "Installation path: $InstallPath" -ForegroundColor Cyan
if (-not $SkipAgentService) {
    Write-Host "Agent service: $agentServiceName" -ForegroundColor Cyan
}
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "1. Sign the kernel driver with a valid certificate" -ForegroundColor Yellow
Write-Host "2. Start the driver with: fltmc load SentinelGuard" -ForegroundColor Yellow
Write-Host "3. Configure settings in: $InstallPath\config\config.toml" -ForegroundColor Yellow
Write-Host "4. Launch the browser UI: powershell -ExecutionPolicy Bypass -File `"$InstallPath\ui\start-web.ps1`"" -ForegroundColor Yellow
Write-Host "5. Open http://localhost:4173 in your browser" -ForegroundColor Yellow
Write-Host "6. Verify the filter is loaded: fltmc" -ForegroundColor Yellow

