cmake_minimum_required(VERSION 3.15)
project(SentinelGuardKernel C)

set(DRIVER_NAME SentinelGuard)
set(DRIVER_SOURCES
    SentinelGuard.c
    Events.c
    Communication.c
)

# Prefer an externally installed WDK CMake package if available.
find_package(wdk QUIET)

if(wdk_FOUND)
    wdk_add_driver(${DRIVER_NAME} ${DRIVER_SOURCES})
    set_target_properties(${DRIVER_NAME} PROPERTIES
        DRIVER_TYPE WDM
        DRIVER_ENTRY DriverEntry
    )
    return()
endif()

message(STATUS "WDK CMake package not found, using built-in WDK fallback")

if(NOT WDK_ROOT)
    if(DEFINED ENV{WDK_ROOT})
        set(WDK_ROOT "$ENV{WDK_ROOT}")
    else()
        set(WDK_ROOT "C:/Program Files (x86)/Windows Kits/10")
    endif()
endif()

if(NOT EXISTS "${WDK_ROOT}/Include")
    message(FATAL_ERROR
        "WDK headers not found under '${WDK_ROOT}'. "
        "Install Windows WDK 10 or pass -DWDK_ROOT=<path>."
    )
endif()

if(NOT WDK_VERSION)
    file(GLOB _wdk_include_versions LIST_DIRECTORIES true "${WDK_ROOT}/Include/10.*")
    set(_wdk_versions)
    foreach(_dir IN LISTS _wdk_include_versions)
        if(IS_DIRECTORY "${_dir}")
            get_filename_component(_ver "${_dir}" NAME)
            list(APPEND _wdk_versions "${_ver}")
        endif()
    endforeach()
    if(NOT _wdk_versions)
        message(FATAL_ERROR
            "No WDK version directories were found under '${WDK_ROOT}/Include'. "
            "Set -DWDK_VERSION=<10.x.y.z>."
        )
    endif()
    list(SORT _wdk_versions COMPARE NATURAL ORDER DESCENDING)
    list(GET _wdk_versions 0 WDK_VERSION)
endif()

set(WDK_KM_INCLUDE "${WDK_ROOT}/Include/${WDK_VERSION}/km")
set(WDK_SHARED_INCLUDE "${WDK_ROOT}/Include/${WDK_VERSION}/shared")
set(WDK_CRT_INCLUDE "${WDK_ROOT}/Include/${WDK_VERSION}/ucrt")
set(WDK_LIB_DIR "${WDK_ROOT}/Lib/${WDK_VERSION}/km/x64")

foreach(_required_path IN ITEMS "${WDK_KM_INCLUDE}" "${WDK_SHARED_INCLUDE}" "${WDK_CRT_INCLUDE}" "${WDK_LIB_DIR}")
    if(NOT EXISTS "${_required_path}")
        message(FATAL_ERROR
            "Required WDK path not found: ${_required_path}\n"
            "Check -DWDK_ROOT and -DWDK_VERSION values."
        )
    endif()
endforeach()

add_library(${DRIVER_NAME} MODULE ${DRIVER_SOURCES})

set_target_properties(${DRIVER_NAME} PROPERTIES
    PREFIX ""
    SUFFIX ".sys"
)

target_include_directories(${DRIVER_NAME} PRIVATE
    "${WDK_KM_INCLUDE}"
    "${WDK_SHARED_INCLUDE}"
    "${WDK_CRT_INCLUDE}"
    "${CMAKE_CURRENT_SOURCE_DIR}"
)

target_compile_definitions(${DRIVER_NAME} PRIVATE
    _AMD64_
    AMD64
    WIN64
    _WIN64
    NTDDI_VERSION=0x0A00000D
    _WIN32_WINNT=0x0A00
    WINVER=0x0A00
)

target_compile_options(${DRIVER_NAME} PRIVATE
    /kernel
    /Zl
    /W4
)

target_link_libraries(${DRIVER_NAME} PRIVATE
    "${WDK_LIB_DIR}/FltMgr.lib"
    "${WDK_LIB_DIR}/NtosKrnl.lib"
    "${WDK_LIB_DIR}/BufferOverflowK.lib"
)

target_link_options(${DRIVER_NAME} PRIVATE
    /SUBSYSTEM:NATIVE
    /DRIVER
    /MANIFEST:NO
    /NODEFAULTLIB
    /ENTRY:DriverEntry
)

message(STATUS "Using WDK_ROOT='${WDK_ROOT}' WDK_VERSION='${WDK_VERSION}'")

