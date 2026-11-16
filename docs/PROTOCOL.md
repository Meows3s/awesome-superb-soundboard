# Serial protocol (v1)

USB CDC-ACM (device class 0x02), VID:PID 2e8a:000a, product string
"Soundboard Macropad", 115200 8N1 (baud is ignored by USB CDC).
ASCII, one message per line, terminated by `\n`.

## Board -> host

| Message   | Meaning                                                  |
|-----------|----------------------------------------------------------|
| `K<NN><d>`| key NN (01..12) went down (d=1) / up (d=0), e.g. `K031` |
| `E+`/`E-` | encoder one detent clockwise / counter-clockwise         |
| `S1`/`S0` | encoder push switch down / up                            |
| `R<hhh>`  | reply to `REQ`: 12-bit key mask, 3 lowercase hex digits  |
| `PONG`    | reply to `PING`                                          |

Key numbering is row-major as on the keypad: key 01 is top-left, 3 columns x
4 rows. NeoPixel N is the LED under key N.

## Host -> board

| Message           | Meaning                                                   |
|-------------------|-----------------------------------------------------------|
| `PING`            | heartbeat; board answers `PONG`. Send at least every ~1 s |
| `COL <NN> <rrggbb>` | set key NN LED colour (hex, no `#`)                     |
| `LBL <NN> <text>` | set key NN OLED label, <= 7 printable ASCII chars; empty clears |
| `VOL <0-100>`     | set the volume shown on the OLED                          |
| `REQ`             | request key mask                                          |
| `LED <n> <0\|1>`  | status LED (legacy)                                       |

The OLED shows "connected" only while a `PING` has been received in the last
2 seconds; otherwise it shows a disconnected banner.

The host owns all state (volume, colours, labels). The knob only emits
`E+`/`E-`; the host applies it and answers with `VOL`.
