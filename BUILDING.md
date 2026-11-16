# Building

The repo has two parts, both driven by one top-level `CMakeLists.txt`:

| Part        | Dir         | Tech                              | Output                      |
|-------------|-------------|-----------------------------------|-----------------------------|
| Firmware    | `firmware/` | Rust, RP2040 (`thumbv6m-none-eabi`) | `soundboard.uf2` / ELF    |
| Desktop app | `desktop/`  | C++17, Qt 6 Widgets               | `soundboard-desktop[.exe]`  |

CMake calls `cargo` for the firmware, so Rust remains the real firmware build tool.

## Quick start

    cmake -S . -B build
    cmake --build build

Build one half only:

    cmake -S . -B build -DBUILD_DESKTOP=OFF     # firmware only
    cmake -S . -B build -DBUILD_FIRMWARE=OFF    # desktop only

## Prerequisites

Common: CMake >= 3.21, Python 3 (used by the UF2 converter, stdlib only).

Firmware (Linux or Windows):

    # rustup: https://rustup.rs  (rust-toolchain.toml pins 1.86.0 and adds the target)
    cargo install flip-link
    cargo install probe-rs-tools --locked   # optional, only for SWD flashing

Also needs an `objcopy` for ELF -> binary: `arm-none-eabi-objcopy`
(Arm GNU Toolchain), or `rustup component add llvm-tools` (`llvm-objcopy`).
Without one the firmware still builds but the `firmware-uf2` target is skipped.

Desktop app, Qt 6.4 or newer, components Widgets (required), SerialPort and
Multimedia (needed for the real features; the skeleton builds without them):

- Windows: install Qt 6 with the online installer (MSVC 2022 64-bit kit, tick
  "Qt Serial Port" and "Qt Multimedia" under Additional Libraries) plus Visual
  Studio 2022 Build Tools (C++). Then, from a "x64 Native Tools" prompt:

      cmake -S . -B build -G "Ninja" -DCMAKE_PREFIX_PATH=C:/Qt/6.8.3/msvc2022_64
      cmake --build build --config Release

  `windeployqt` runs automatically after the build, so
  `build\desktop\soundboard-desktop.exe` starts directly.
- Ubuntu/Debian:

      sudo -S -p '' apt install qt6-base-dev qt6-serialport-dev qt6-multimedia-dev

## Firmware targets

| Target            | What it does                                              |
|-------------------|-----------------------------------------------------------|
| `firmware`        | cargo build (release) -> ELF                              |
| `firmware-uf2`    | ELF -> `build/firmware/soundboard.uf2`                    |
| `firmware-flash`  | flash + run via probe-rs (needs an SWD probe)             |
| `firmware-test`   | host-side unit tests (protocol + encoder logic)           |

Run with `cmake --build build --target <name>`.

### Deploying to the macropad (no probe needed)

1. Unplug the macropad. Hold the **encoder knob (BOOT)** button, plug in USB
   (or hold it and tap the reset button). A drive named `RPI-RP2` appears.
2. Copy `build/firmware/soundboard.uf2` onto that drive. The board reboots
   into the new firmware by itself.
3. Check it enumerates: Linux `ls /dev/ttyACM*`, Windows Device Manager
   -> Ports (COM). Then `screen /dev/ttyACM0 115200` and press keys; you
   should see lines like `K031` / `K030` and `E+` / `E-`.

### Without CMake

    cargo build -p soundboard-firmware --release --target thumbv6m-none-eabi
    cargo test  -p soundboard-firmware

Always pass `--target`; a bare `cargo build` at the workspace root would try
to compile the firmware for the host.

Pushes to GitHub also build both halves in CI (`.github/workflows/build.yml`);
the Windows job uploads a ready-to-run `soundboard-desktop-windows` artifact and
the firmware job uploads `soundboard.uf2`.

## Desktop app

    ./build/desktop/soundboard-desktop        # Linux
    build\desktop\soundboard-desktop.exe     # Windows
    ctest --test-dir build/desktop            # unit tests (protocol, mixer)

On Linux, if Qt was installed outside the system (e.g. with `aqt` into `~/Qt`)
pass `-DCMAKE_PREFIX_PATH=~/Qt/6.8.3/gcc_64` and export
`LD_LIBRARY_PATH=~/Qt/6.8.3/gcc_64/lib`. Your user must be in the `dialout`
group to open `/dev/ttyACM0`.

### Using it

1. Install VB-Cable (https://vb-audio.com/Cable/). In the app choose
   output = "CABLE Input"; in Discord set the microphone to "CABLE Output".
   (Auto-selected on first run if present.)
2. Click a key to assign a sound file, a label (max 7 chars, shown on the
   macropad OLED) and a colour (shown on the key's LED). Right-click a key to
   test-play it.
3. Turn the knob to change the effect volume (mic level is untouched); press
   the knob to stop all sounds.
4. Monitor: effects are also played to a second output so you can hear them.
   Pick your headphones/speakers under "Monitor output". It uses the same
   master volume as the knob and never includes your mic. It must differ from
   the cable output (the app disables it and warns otherwise). If VB-Cable is
   your Windows default output, choose the monitor device explicitly rather
   than "System default".
5. The window banner shows the link state. "Found but cannot open port" means
   another program (e.g. a serial monitor) holds the COM port; close it.
6. Config lives in the per-user app-config directory as `config.json`
   (Windows: `%APPDATA%`, Linux: `~/.config`).

The wire protocol is documented in `docs/PROTOCOL.md`.

## Troubleshooting

- **COM port visible but won't open (Windows):** reflash the current firmware
  (it declares the CDC device class and USB strings that Windows' `usbser`
  needs). If an old entry is cached, unplug the board and uninstall the device
  in Device Manager, then replug.
- **OLED upside down:** flip `ROTATE_180` in `firmware/src/display.rs`
  (currently `true`, correct for the normal desk orientation).
- **Can't push workflow changes with the GitHub App:** the App needs the
  *Workflows* repository permission (not just *Actions*), approved on the
  installation.
