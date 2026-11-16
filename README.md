# awesome-superb-soundboard (a.s.s.)

Firmware and an accompanying host app for an **Adafruit RP2040 Macropad**
soundboard.

* `firmware/` - bare-metal Rust firmware for the macropad (RP2040): scans the
  12 keys + rotary encoder and talks to the host over USB CDC serial.
* `desktop/` - C++/Qt 6 desktop app (Windows; Linux optional) that assigns
  sounds to keys, sets key colours and mixes into a VB-Cable virtual mic.

## Getting the app

**Windows (prebuilt):** every push to `main` is built by GitHub Actions.

1. Open the repo's **Actions** tab and click the latest successful
   **build** run.
2. Under **Artifacts** at the bottom, download `soundboard-desktop-windows`
   (you must be logged in to GitHub; artifacts expire after 90 days).
3. Unzip it and run `soundboard-desktop.exe` (Qt libraries are bundled).
4. Install [VB-Cable](https://vb-audio.com/Cable/), choose "CABLE Input" as
   the app's output, and set Discord's microphone to "CABLE Output".

**Macropad firmware:** the same run has a `soundboard-firmware-uf2` artifact.
Hold the knob while plugging the pad in, then copy the `.uf2` onto the
`RPI-RP2` drive that appears.

**Build it yourself** (Windows or Linux): see [BUILDING.md](BUILDING.md) for
prerequisites, flashing and troubleshooting.

## Status

Working end to end on hardware and Windows:

- Firmware: 12 keys, encoder + push switch, per-key NeoPixel colours, OLED
  (connection state, volume, key labels), PING/PONG heartbeat. 32 host-side
  logic tests.
- Desktop app (Qt 6, dark theme): per-key sound/label/colour assignment,
  encoder-controlled effect volume, mic + effects mixed into VB-Cable, local
  monitor output sharing the master volume. Protocol and mixer unit tests; CI
  builds firmware (UF2) and the Windows exe on every push.
- Linux desktop build works but is less tested than Windows.
