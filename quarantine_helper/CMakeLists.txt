cmake_minimum_required(VERSION 3.20)
project(SentinelGuardQuarantineHelper CXX)

set(CMAKE_CXX_STANDARD 17)

add_executable(quarantine_helper src/main.cpp)
target_link_libraries(quarantine_helper PRIVATE ntdll)

# Ensure we link against Windows libraries
target_link_libraries(quarantine_helper PRIVATE
    kernel32
)

# Set output name
set_target_properties(quarantine_helper PROPERTIES
    OUTPUT_NAME "quarantine_helper"
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin"
)
