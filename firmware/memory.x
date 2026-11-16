MEMORY
{
    /* RP2040 flash: 2 MiB starting at 0x1000_0000 (stage-2 boot at 0x100). */
    BOOT2 : ORIGIN = 0x10000000, LENGTH = 0x100
    FLASH : ORIGIN = 0x10000100, LENGTH = 2048K - 0x100
    RAM   : ORIGIN = 0x20000000, LENGTH = 256K
}

/* Default stack size (used by flip-link / cortex-m-rt). */
STACK_SIZE = 0x1000;

/* Place the 256-byte second-stage bootloader at the start of flash. Without
   this the RP2040 ROM finds no valid boot2 and never runs the firmware. */
SECTIONS {
    .boot2 ORIGIN(BOOT2) :
    {
        KEEP(*(.boot2));
    } > BOOT2
} INSERT BEFORE .text;
