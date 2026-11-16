cmake_minimum_required(VERSION 3.21)
project(awesome_superb_soundboard LANGUAGES C CXX)

# Top-level build for both halves of the project:
#   firmware/  Rust, RP2040 (thumbv6m-none-eabi), driven through cargo
#   desktop/   C++17 + Qt 6 Widgets desktop app (Windows, Linux optional)
#
# Either half can be switched off:
#   -DBUILD_FIRMWARE=OFF   -DBUILD_DESKTOP=OFF
option(BUILD_FIRMWARE "Build the RP2040 firmware (needs cargo + flip-link)" ON)
option(BUILD_DESKTOP  "Build the Qt desktop app (needs Qt 6)" ON)

if(BUILD_FIRMWARE)
  add_subdirectory(firmware)
endif()
if(BUILD_DESKTOP)
  add_subdirectory(desktop)
endif()
