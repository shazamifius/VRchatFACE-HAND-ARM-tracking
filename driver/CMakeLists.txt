# Builds driver_vrcbridge.dll — the VRChat Bridge Hub virtual HMD SteamVR driver.
#
# A SteamVR driver only needs openvr_driver.h (vendored in headers/) and must be
# named driver_<name>.dll with no "lib" prefix. We target 64-bit Windows; the
# DLL is installed under <driver root>/bin/win64/ next to driver.vrdrivermanifest.
cmake_minimum_required(VERSION 3.15)
project(vrcbridge_driver LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(driver_vrcbridge SHARED src/driver_main.cpp)
target_include_directories(driver_vrcbridge PRIVATE headers)

# SteamVR looks for "driver_<name>.dll" — strip the default lib prefix.
set_target_properties(driver_vrcbridge PROPERTIES PREFIX "")
