cmake_minimum_required(VERSION 3.15)
project(SentinelGuardKernel C)

# Find WDK
find_package(wdk REQUIRED)

# Driver source files
set(DRIVER_SOURCES
    SentinelGuard.c
    SentinelGuard.h
    Events.c
    Events.h
    Communication.c
    Communication.h
)

# Driver configuration
set(DRIVER_NAME "SentinelGuard")
set(DRIVER_TARGET_TYPE DRIVER)

# Include WDK build
include(${WDK_ROOT}/build/nmake/msvc.mak)

# Add driver
wdk_add_driver(${DRIVER_NAME} ${DRIVER_SOURCES})

# Set driver properties
set_target_properties(${DRIVER_NAME} PROPERTIES
    DRIVER_TYPE WDM
    DRIVER_ENTRY DriverEntry
)

