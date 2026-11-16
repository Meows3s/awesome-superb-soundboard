# Firmware build: cargo does the compiling, CMake wires up the outputs.
#   cmake --build build --target firmware        -> ELF
#   cmake --build build --target firmware-uf2    -> soundboard.uf2 (drag onto RPI-RP2)
#   cmake --build build --target firmware-flash  -> flash via probe-rs (SWD probe)
#   cmake --build build --target firmware-test   -> host-side logic tests
find_program(CARGO cargo HINTS "$ENV{HOME}/.cargo/bin" "$ENV{USERPROFILE}/.cargo/bin")
find_program(FLIP_LINK flip-link HINTS "$ENV{HOME}/.cargo/bin" "$ENV{USERPROFILE}/.cargo/bin")
find_program(PROBE_RS probe-rs HINTS "$ENV{HOME}/.cargo/bin" "$ENV{USERPROFILE}/.cargo/bin")
find_program(OBJCOPY NAMES arm-none-eabi-objcopy llvm-objcopy rust-objcopy objcopy)
find_package(Python3 REQUIRED COMPONENTS Interpreter)

if(NOT CARGO)
  message(FATAL_ERROR "cargo not found. Install Rust via https://rustup.rs "
                      "or configure with -DBUILD_FIRMWARE=OFF")
endif()
if(NOT FLIP_LINK)
  message(FATAL_ERROR "flip-link not found. Run: cargo install flip-link")
endif()

set(FW_TARGET thumbv6m-none-eabi)
set(FW_PROFILE release)
set(WORKSPACE_DIR  ${CMAKE_CURRENT_SOURCE_DIR}/..)
set(FW_CARGO_DIR   ${CMAKE_CURRENT_BINARY_DIR}/cargo-target)
set(FW_ELF         ${FW_CARGO_DIR}/${FW_TARGET}/${FW_PROFILE}/soundboard_firmware)
set(FW_BIN         ${CMAKE_CURRENT_BINARY_DIR}/soundboard.bin)
set(FW_UF2         ${CMAKE_CURRENT_BINARY_DIR}/soundboard.uf2)

file(GLOB_RECURSE FW_SOURCES CONFIGURE_DEPENDS ${CMAKE_CURRENT_SOURCE_DIR}/src/*.rs)

# Always invoke cargo (it does its own up-to-date check); the ELF is a byproduct.
add_custom_target(firmware ALL
  COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${FW_CARGO_DIR}
          ${CARGO} build -p soundboard-firmware --${FW_PROFILE} --target ${FW_TARGET}
  WORKING_DIRECTORY ${WORKSPACE_DIR}
  BYPRODUCTS ${FW_ELF}
  COMMENT "cargo build (firmware, ${FW_TARGET})"
  USES_TERMINAL)

if(OBJCOPY)
  add_custom_target(firmware-uf2 ALL
    COMMAND ${OBJCOPY} -O binary ${FW_ELF} ${FW_BIN}
    COMMAND ${Python3_EXECUTABLE} ${WORKSPACE_DIR}/tools/bin2uf2.py ${FW_BIN} ${FW_UF2}
    DEPENDS firmware
    BYPRODUCTS ${FW_BIN} ${FW_UF2}
    COMMENT "Converting ELF -> UF2")
else()
  message(WARNING "No objcopy found; firmware-uf2 target disabled "
                  "(install arm-none-eabi-binutils or llvm-tools)")
endif()

if(PROBE_RS)
  add_custom_target(firmware-flash
    COMMAND ${PROBE_RS} run --chip RP2040 ${FW_ELF}
    DEPENDS firmware USES_TERMINAL
    COMMENT "Flashing via probe-rs")
endif()

# Pure protocol/encoder logic runs on the host, no hardware needed.
add_custom_target(firmware-test
  COMMAND ${CARGO} test -p soundboard-firmware
  WORKING_DIRECTORY ${WORKSPACE_DIR} USES_TERMINAL)
