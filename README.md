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

Firmware: serial events, NeoPixels, OLED (SH1106) and heartbeat implemented;
32 host-side logic tests pass. Key/encoder input confirmed on hardware; OLED
and NeoPixels are **not yet verified on hardware**.
Desktop app: Qt 6 UI, serial link, mixer + VB-Cable routing; unit tests pass on
Linux, Windows build via CI.
