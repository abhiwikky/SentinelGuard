# SentinelGuard Kernel Minifilter Driver

## Overview

Windows minifilter driver that intercepts file system operations and streams events to the user-mode agent.

## Requirements

- Windows Driver Kit (WDK) 10
- Visual Studio 2019 or later
- Windows SDK

## Building

```powershell
# From kernel directory
mkdir build
cd build
cmake .. -G "Visual Studio 16 2019" -A x64
cmake --build . --config Release
```

## Installation

The driver must be signed and installed with administrator privileges:

```powershell
# Install driver
sc create SentinelGuard type= kernel binPath= "C:\Program Files\SentinelGuard\SentinelGuard.sys"
sc start SentinelGuard
```

## Architecture

The driver uses FltMgr (Filter Manager) to:
- Intercept file create, read, write, rename, delete operations
- Monitor directory-level operations
- Detect VSS deletion attempts
- Stream events via ALPC to user-mode agent

## Security

- Code signing required for kernel mode
- Tamper detection
- Secure communication channel

