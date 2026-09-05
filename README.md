# awesome-superb-soundboard (a.s.s.)

Firmware and an accompanying host app for an **Adafruit RP2040 Macropad**
soundboard.

* `firmware/` — bare-metal Rust firmware that runs on the macropad (RP2040,
  `thumbv6m-none-eabi`). Brings up the clocks, scans the 12 keys + rotary
  encoder, drives the status LED, and logs events over defmt/RTT.
* `app/` — desktop companion app (CLI stub) that will talk to the board over
  USB to push sample banks and read encoder/key state.

## Toolchain setup

```sh
# install rust + the embedded target
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add thumbv6m-none-eabi
cargo install flip-link          # linker with stack-overflow guard
cargo install probe-rs --features cli --locked   # flash/debug
```

`rust-toolchain.toml` pins the channel and auto-installs `rustfmt`/`clippy`
and the `thumbv6m-none-eabi` target.

## Build & flash

```sh
# cross-compile the firmware
cargo build -p soundboard-firmware

# build + flash + start a debug session (needs a probe attached)
cargo run -p soundboard-firmware

# watch defmt logs over RTT
probe-rs attach --chip RP2040
```

## Pushing changes

The repo's local git config uses the `github-app` GitHub App
installation token (via `a helper script`) for HTTPS
pushes — no personal access token needed:

```sh
git push origin main
```

## Status

Scaffold only: hardware bring-up (clocks, key scan, LED, defmt logging)
compiles and is the verified baseline. The sound engine (sample playback,
NeoPixel feedback, USB-HID to the host app) and the `app/` CLI are not yet
implemented — see `firmware/Cargo.toml` NOTES for the deferred dependency
decisions (ws2812-pio / smart-leds and embassy-rp require a newer HAL
generation than the `adafruit-macropad` 0.8 BSP).
