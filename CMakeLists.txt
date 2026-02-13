cmake_minimum_required(VERSION 3.20)

project(VRChatUniversalVideoBridge VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# --- Vcpkg Integration ---
# Vcpkg toolchain file should be passed via command line or environment variable
# e.g., -DCMAKE_TOOLCHAIN_FILE=[path to vcpkg]/scripts/buildsystems/vcpkg.cmake

# --- Dependencies ---
find_package(glm CONFIG REQUIRED)
find_package(concurrentqueue CONFIG REQUIRED)
find_package(spdlog CONFIG REQUIRED)
find_package(OpenCV REQUIRED)
find_package(imgui CONFIG REQUIRED)
find_package(onnxruntime CONFIG REQUIRED)
find_package(glad CONFIG REQUIRED)
find_package(glfw3 CONFIG REQUIRED)

# QR Code library
find_path(QRENCODE_INCLUDE_DIR NAMES qrencode.h)
find_library(QRENCODE_LIBRARY_RELEASE NAMES qrencode)
find_library(QRENCODE_LIBRARY_DEBUG NAMES qrencoded)

# Manually set libraries based on build type
if(QRENCODE_LIBRARY_DEBUG AND QRENCODE_LIBRARY_RELEASE)
  set(QRENCODE_LIBRARIES optimized ${QRENCODE_LIBRARY_RELEASE} debug ${QRENCODE_LIBRARY_DEBUG})
elseif(QRENCODE_LIBRARY_RELEASE)
  set(QRENCODE_LIBRARIES ${QRENCODE_LIBRARY_RELEASE})
elseif(QRENCODE_LIBRARY_DEBUG)
  set(QRENCODE_LIBRARIES ${QRENCODE_LIBRARY_DEBUG})
endif()

# GLM Configuration
add_compile_definitions(GLM_ENABLE_EXPERIMENTAL)

# --- Testing ---
enable_testing()
find_package(GTest CONFIG REQUIRED)

# --- Sources ---
file(GLOB_RECURSE SOURCES "src/*.cpp" "src/*.h" "src/*.hpp")

# Exclude tests and main.cpp (added explicitly)
list(FILTER SOURCES EXCLUDE REGEX "src/tests/.*")
list(FILTER SOURCES EXCLUDE REGEX "src/main.cpp")

# --- Main Executable ---
add_executable(${PROJECT_NAME} 
    src/main.cpp
    ${SOURCES}
)

# --- Include Directories ---
target_include_directories(${PROJECT_NAME} PRIVATE src)

# --- Linking ---
target_link_libraries(${PROJECT_NAME} PRIVATE
    glm::glm
    concurrentqueue::concurrentqueue
    spdlog::spdlog
    ${OpenCV_LIBS}
    imgui::imgui
    onnxruntime::onnxruntime
    ws2_32
    glad::glad
    glfw
    opengl32
    ${QRENCODE_LIBRARIES}
)

# Include qrencode headers
target_include_directories(${PROJECT_NAME} PRIVATE ${QRENCODE_INCLUDE_DIR})

# --- Unit Tests ---
enable_testing()
add_executable(unit_tests 
    src/tests/test_Biomech.cpp 
    src/tests/test_Core.cpp 
    src/tests/test_Network.cpp 
    src/tests/test_Vision.cpp
)
target_link_libraries(unit_tests PRIVATE 
    GTest::gtest 
    GTest::gtest_main
    ${OpenCV_LIBS}
    onnxruntime::onnxruntime
    glm::glm
    spdlog::spdlog
    concurrentqueue::concurrentqueue
)

add_test(NAME UnitTests COMMAND unit_tests)
