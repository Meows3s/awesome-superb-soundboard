#!/usr/bin/env python3
"""Convert a raw RP2040 flash image to UF2 (no external dependencies).

usage: bin2uf2.py input.bin output.uf2 [base_address_hex]
Default base address is 0x10000000 (start of RP2040 XIP flash, where boot2 lives).
"""
import struct
import sys

RP2040_FAMILY_ID = 0xE48BFF56
MAGIC0, MAGIC1, MAGIC_END = 0x0A324655, 0x9E5D5157, 0x0AB16F30
FLAG_FAMILY_ID_PRESENT = 0x00002000
PAYLOAD = 256


def convert(data: bytes, base: int) -> bytes:
    blocks = [data[i:i + PAYLOAD] for i in range(0, len(data), PAYLOAD)]
    out = bytearray()
    for n, chunk in enumerate(blocks):
        chunk = chunk.ljust(476, b"\x00")  # data area is always 476 bytes
        out += struct.pack(
            "<IIIIIIII", MAGIC0, MAGIC1, FLAG_FAMILY_ID_PRESENT,
            base + n * PAYLOAD, PAYLOAD, n, len(blocks), RP2040_FAMILY_ID)
        out += chunk + struct.pack("<I", MAGIC_END)
    return bytes(out)


def main() -> int:
    if len(sys.argv) not in (3, 4):
        print(__doc__)
        return 2
    base = int(sys.argv[3], 16) if len(sys.argv) == 4 else 0x10000000
    data = open(sys.argv[1], "rb").read()
    open(sys.argv[2], "wb").write(convert(data, base))
    print(f"{sys.argv[2]}: {len(data)} bytes -> {-(-len(data)//PAYLOAD)} UF2 blocks")
    return 0


if __name__ == "__main__":
    sys.exit(main())
