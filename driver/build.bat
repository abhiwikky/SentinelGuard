@echo off
REM SentinelGuard Kernel Driver Build Script
REM Requires VS2022 and WDK 10.0.26100.0

setlocal enabledelayedexpansion

REM ─── Configuration ────────────────────────────────────────────────
set "VS_PATH=C:\Program Files\Microsoft Visual Studio\18\Community"
set "WDK_ROOT=C:\Program Files (x86)\Windows Kits\10"
set "WDK_VER=10.0.26100.0"
set "CONFIG=Release"
set "ARCH=x64"

REM ─── Setup VS Environment ────────────────────────────────────────
call "%VS_PATH%\VC\Auxiliary\Build\vcvarsall.bat" amd64 %WDK_VER%
if errorlevel 1 (
    echo ERROR: Failed to set up VS environment
    exit /b 1
)

REM ─── Paths ────────────────────────────────────────────────────────
set "WDK_INC=%WDK_ROOT%\Include\%WDK_VER%"
set "WDK_LIB=%WDK_ROOT%\Lib\%WDK_VER%"
set "OUT_DIR=%~dp0build\%CONFIG%\%ARCH%"

if not exist "%OUT_DIR%" mkdir "%OUT_DIR%"

REM ─── Compiler Flags for Kernel Mode ──────────────────────────────
set CL_FLAGS=/nologo /c /Zi /W4 /WX- /Od
set CL_FLAGS=%CL_FLAGS% /D _WIN64 /D _AMD64_ /D AMD64
set CL_FLAGS=%CL_FLAGS% /D _KERNEL_MODE /D NTDDI_VERSION=0x0A000000
set CL_FLAGS=%CL_FLAGS% /D _WIN32_WINNT=0x0A00
set CL_FLAGS=%CL_FLAGS% /D WINVER=0x0A00
set CL_FLAGS=%CL_FLAGS% /D WINNT=1
set CL_FLAGS=%CL_FLAGS% /D POOL_NX_OPTIN=1
set CL_FLAGS=%CL_FLAGS% /kernel /GS- /Gy
set CL_FLAGS=%CL_FLAGS% /I "%WDK_INC%\km"
set CL_FLAGS=%CL_FLAGS% /I "%WDK_INC%\shared"
set CL_FLAGS=%CL_FLAGS% /I "%WDK_INC%\ucrt"
set CL_FLAGS=%CL_FLAGS% /I "%~dp0src"
set CL_FLAGS=%CL_FLAGS% /Fo"%OUT_DIR%\\"

REM ─── Link Flags for Kernel Mode ──────────────────────────────────
set LINK_FLAGS=/nologo /DEBUG /DRIVER /SUBSYSTEM:NATIVE /ENTRY:DriverEntry
set LINK_FLAGS=%LINK_FLAGS% /LIBPATH:"%WDK_LIB%\km\x64"
set LINK_FLAGS=%LINK_FLAGS% /LIBPATH:"%WDK_LIB%\ucrt\x64"
set LINK_FLAGS=%LINK_FLAGS% fltMgr.lib ntstrsafe.lib ntoskrnl.lib hal.lib wmilib.lib BufferOverflowFastFailK.lib

REM ─── Compile ──────────────────────────────────────────────────────
echo.
echo ═══════════════════════════════════════════════════
echo   SentinelGuard Kernel Driver Build
echo ═══════════════════════════════════════════════════
echo.
echo [BUILD] Compiling driver sources...

cl.exe %CL_FLAGS% src\driver.c
if errorlevel 1 (
    echo ERROR: Failed to compile driver.c
    exit /b 1
)
echo [BUILD]   driver.c - OK

cl.exe %CL_FLAGS% src\communication.c
if errorlevel 1 (
    echo ERROR: Failed to compile communication.c
    exit /b 1
)
echo [BUILD]   communication.c - OK

cl.exe %CL_FLAGS% src\operations.c
if errorlevel 1 (
    echo ERROR: Failed to compile operations.c
    exit /b 1
)
echo [BUILD]   operations.c - OK

REM ─── Link ─────────────────────────────────────────────────────────
echo [BUILD] Linking sentinelguard.sys...

link.exe %LINK_FLAGS% /OUT:"%OUT_DIR%\sentinelguard.sys" "%OUT_DIR%\driver.obj" "%OUT_DIR%\communication.obj" "%OUT_DIR%\operations.obj"
if errorlevel 1 (
    echo ERROR: Failed to link sentinelguard.sys
    exit /b 1
)

echo.
echo [BUILD] ═══════════════════════════════════════════
echo [BUILD]   Build successful!
echo [BUILD]   Output: %OUT_DIR%\sentinelguard.sys
echo [BUILD] ═══════════════════════════════════════════
echo.

endlocal
