cmake_minimum_required(VERSION 3.15)
project(Quarantine)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_executable(quarantine
    quarantine.cpp
    quarantine.h
    process_control.cpp
    process_control.h
)

target_link_libraries(quarantine
    ntdll
)

if(WIN32)
    set_target_properties(quarantine PROPERTIES
        LINK_FLAGS "/SUBSYSTEM:CONSOLE"
    )
endif()

