# awesome-superb-soundboard (a.s.s.)

Firmware and an accompanying host app for an **Adafruit RP2040 Macropad**
soundboard.

* `firmware/` - bare-metal Rust firmware for the macropad (RP2040): scans the
  12 keys + rotary encoder and talks to the host over USB CDC serial.
* `desktop/` - C++/Qt 6 desktop app (Windows; Linux optional) that assigns
  sounds to keys, sets key colours and mixes into a VB-Cable virtual mic.

See [BUILDING.md](BUILDING.md) for build, flash and deploy instructions.

## Pushing changes

The repo's local git config uses the `github-app` GitHub App
installation token (via `a helper script`) for HTTPS
pushes — no personal access token needed:

```sh
git push origin main
```

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
