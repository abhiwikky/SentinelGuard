# SentinelGuard Installation Script
# Requires Administrator privileges

param(
    [string]$InstallPath = "C:\Program Files\SentinelGuard",
    [switch]$SkipDriver = $false
)

$ErrorActionPreference = "Stop"

Write-Host "SentinelGuard Installation Script" -ForegroundColor Green
Write-Host "=================================" -ForegroundColor Green

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

# Copy agent executable
Write-Host "Installing agent..." -ForegroundColor Yellow
Copy-Item "agent\target\release\sentinelguard-agent.exe" -Destination "$InstallPath\sentinelguard-agent.exe" -Force

# Copy quarantine module
Write-Host "Installing quarantine module..." -ForegroundColor Yellow
Copy-Item "quarantine\build\Release\quarantine.exe" -Destination "$InstallPath\quarantine.exe" -Force

# Copy UI
Write-Host "Installing UI..." -ForegroundColor Yellow
Copy-Item "ui\dist\*" -Destination "$InstallPath\ui\" -Recurse -Force

# Copy ML model
Write-Host "Installing ML model..." -ForegroundColor Yellow
Copy-Item "ml\models\*.onnx" -Destination "$InstallPath\models\" -Force

# Copy configuration
Write-Host "Installing configuration..." -ForegroundColor Yellow
Copy-Item "agent\config\config.toml" -Destination "$InstallPath\config\config.toml" -Force

# Install kernel driver
if (-not $SkipDriver) {
    Write-Host "Installing kernel driver..." -ForegroundColor Yellow
    $driverPath = "$InstallPath\SentinelGuard.sys"
    Copy-Item "kernel\build\Release\SentinelGuard.sys" -Destination $driverPath -Force
    
    # Install driver service
    $serviceName = "SentinelGuard"
    $existingService = Get-Service -Name $serviceName -ErrorAction SilentlyContinue
    
    if ($existingService) {
        Write-Host "Stopping existing driver service..." -ForegroundColor Yellow
        Stop-Service -Name $serviceName -Force -ErrorAction SilentlyContinue
        sc.exe delete $serviceName | Out-Null
    }
    
    Write-Host "Creating driver service..." -ForegroundColor Yellow
    sc.exe create $serviceName type= kernel binPath= "$driverPath" start= demand
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Driver service created successfully" -ForegroundColor Green
    } else {
        Write-Host "WARNING: Failed to create driver service. Driver may need to be signed." -ForegroundColor Red
    }
}

# Install agent as Windows service
Write-Host "Installing agent service..." -ForegroundColor Yellow
$agentServiceName = "SentinelGuardAgent"
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
    Start-Service -Name $agentServiceName
    Write-Host "Agent service started" -ForegroundColor Green
} else {
    Write-Host "ERROR: Failed to create agent service" -ForegroundColor Red
    exit 1
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
Write-Host "Agent service: $agentServiceName" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "1. Sign the kernel driver with a valid certificate" -ForegroundColor Yellow
Write-Host "2. Start the driver service: sc start SentinelGuard" -ForegroundColor Yellow
Write-Host "3. Configure settings in: $InstallPath\config\config.toml" -ForegroundColor Yellow
Write-Host "4. Launch the UI from: $InstallPath\ui\" -ForegroundColor Yellow

