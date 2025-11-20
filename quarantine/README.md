# SentinelGuard Quarantine Module

## Overview

C++ module for suspending and isolating malicious processes detected by the agent.

## Building

```powershell
mkdir build
cd build
cmake .. -G "Visual Studio 16 2019" -A x64
cmake --build . --config Release
```

## Usage

```powershell
# Suspend a process
quarantine.exe --suspend <pid>

# Kill a process
quarantine.exe --kill <pid>
```

## Features

- Process suspension via NtSuspendProcess
- Process termination
- File handle blocking (via suspension)
- ACL modification (read-only files)

## Requirements

- Administrator privileges
- Windows 10 or later

