# Builds vrcbridge_probe.exe - a small OpenVR *client* tool that dumps the live
# tracked-device state from a running SteamVR (see probe.cpp). It links the
# vendored openvr_api import lib and needs openvr_api.dll at runtime (copied
# next to the exe from SteamVR by the build/DIAGNOSE script).
cmake_minimum_required(VERSION 3.15)
project(vrcbridge_probe LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_executable(vrcbridge_probe probe.cpp)
target_include_directories(vrcbridge_probe PRIVATE headers)
target_link_libraries(vrcbridge_probe PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/lib/openvr_api.lib")
