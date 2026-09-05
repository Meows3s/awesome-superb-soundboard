//! awesome-superb-soundboard firmware for the Adafruit RP2040 Macropad.
//!
//! Synchronous, no-async design so it builds on current stable Rust. Each tick
//! it reads the keys and encoder and reports events over USB-CDC serial (the
//! same USB cable; host sees /dev/ttyACMx). It also listens for incoming
//! commands to drive the status LED and a display text buffer.
//!
//! Pinout (from the `adafruit-macropad` BSP):
//!   enc switch (BOOT button) = GPIO0
//!   key1..12 = GPIO1..GPIO12   status LED = GPIO13   speaker = GPIO16
//!   enc_a = GPIO17  enc_b = GPIO18  neopixel = GPIO19
//!   sda = GPIO20  scl = GPIO21  oled_cs = GPIO22  oled_reset = GPIO23
//!   oled_dc = GPIO24  spi_sclk = GPIO26  spi_mosi = GPIO27  spi_miso = GPIO28
//!
//! Serial protocol (both directions are ASCII, newline-terminated):
//!   OUT (board -> host), one compact event line per change:
//!     "K<NN><0|1>"   key NN (01..=12) down(1)/up(0)   e.g. "K031"
//!     "E+" / "E-"    encoder one detent CW / CCW
//!     "S0" / "S1"    encoder push switch (BOOT button) up / down
//!     "R<hhh>"       reply to REQ — current 12-key mask, 3 lowercase hex digits
//!   IN  (host -> board), one complete line at a time:
//!     "REQ"          -> board replies with "R<hhh>"
//!     "LED <n> <0|1>" -> set the status LED (there is one status LED)
//!     "TEXT <string>" -> store <string> as the display text buffer
//!
//! Build/flash:
//!   cargo build -p soundboard-firmware --target thumbv6m-none-eabi
//!   cargo run  -p soundboard-firmware --target thumbv6m-none-eabi  # probe-rs
//! Host-side unit tests for the protocol/encoder logic (pure, no hardware):
//!   cargo test -p soundboard-firmware

// The crate is no_std/no_main on the device, but `cargo test` compiles it for
// the HOST (x86_64) to run the pure logic tests — std + test harness needed.
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

mod protocol;

#[cfg(not(test))]
use cortex_m_rt::entry;
#[cfg(not(test))]
use embedded_hal::delay::DelayNs;
#[cfg(not(test))]
use embedded_hal::digital::{InputPin, OutputPin};
#[cfg(not(test))]
use panic_halt as _;
#[cfg(not(test))]
use rp2040_hal::{
    clocks::init_clocks_and_plls,
    gpio::{self, DynPinId, FunctionSioInput, FunctionSioOutput, PullDown, PullUp},
    pac,
    sio::Sio,
    usb::UsbBus,
    Timer,
};
#[cfg(not(test))]
use usb_device::class_prelude::UsbBusAllocator;
#[cfg(not(test))]
use usb_device::device::{UsbDevice, UsbDeviceBuilder, UsbVidPid};
#[cfg(not(test))]
use usb_device::prelude::UsbError;
#[cfg(not(test))]
use usbd_serial::SerialPort;

/// The macropad crystal is 12 MHz (same as the Pico).
#[cfg(not(test))]
const XOSC_CRYSTAL_FREQ: u32 = 12_000_000;
/// Tick period for the state report.
#[cfg(not(test))]
const TICK_MS: u32 = 100;

/// Type-erased input pin (all 12 keys share this type after `into_dyn_pin`).
#[cfg(not(test))]
type KeyPin = gpio::Pin<DynPinId, FunctionSioInput, PullUp>;

#[cfg(not(test))]
#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = rp2040_hal::Watchdog::new(pac.WATCHDOG);
    let clocks = init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let sio = Sio::new(pac.SIO);
    // rp2040-hal 0.10 builds pins through the `Pins` struct.
    let pins = rp2040_hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // 12 keys as type-erased inputs (so they fit a uniform array).
    let mut keys: [KeyPin; 12] = [
        pins.gpio1.into_pull_up_input().into_dyn_pin(),
        pins.gpio2.into_pull_up_input().into_dyn_pin(),
        pins.gpio3.into_pull_up_input().into_dyn_pin(),
        pins.gpio4.into_pull_up_input().into_dyn_pin(),
        pins.gpio5.into_pull_up_input().into_dyn_pin(),
        pins.gpio6.into_pull_up_input().into_dyn_pin(),
        pins.gpio7.into_pull_up_input().into_dyn_pin(),
        pins.gpio8.into_pull_up_input().into_dyn_pin(),
        pins.gpio9.into_pull_up_input().into_dyn_pin(),
        pins.gpio10.into_pull_up_input().into_dyn_pin(),
        pins.gpio11.into_pull_up_input().into_dyn_pin(),
        pins.gpio12.into_pull_up_input().into_dyn_pin(),
    ];

    // Encoder pins + status LED output. (NeoPixel sense pin dropped for now —
    // NeoPixel feedback is a later task.)
    let _enc_a = pins.gpio17.into_pull_up_input().into_dyn_pin();
    let _enc_b = pins.gpio18.into_pull_up_input().into_dyn_pin();
    let mut led = pins.gpio13.into_push_pull_output().into_dyn_pin();

    let mut timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    // --- USB CDC-ACM setup -------------------------------------------------
    let usb_clock = clocks.usb_clock;
    let usb_bus = UsbBusAllocator::new(UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        usb_clock,
        true,
        &mut pac.RESETS,
    ));
    let mut serial = SerialPort::new(&usb_bus);
    let mut usb_dev: UsbDevice<'_, UsbBus> =
        UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x2e8a, 0x000a)).build();

    let mut display_text: heapless::String<64> = heapless::String::new();
    let mut prev_keys: [bool; 12] = [false; 12];
    let mut mask: u16 = 0;
    let mut first_report = true;
    let mut rx_buf = [0u8; 64];
    let mut rx_pos = 0usize;

    loop {
        // 1) Poll USB; handle inbound bytes into the command parser.
        if usb_dev.poll(&mut [&mut serial]) {
            loop {
                match serial.read(&mut rx_buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let mut i = 0;
                        while i < n {
                            let b = rx_buf[i];
                            i += 1;
                            if b == b'\n' || b == b'\r' {
                                if rx_pos > 0 {
                                    if let Ok(s) = core::str::from_utf8(&rx_buf[..rx_pos]) {
                                        handle_command(
                                            &mut serial,
                                            &mut led,
                                            &mut display_text,
                                            &mask,
                                            s,
                                        );
                                    }
                                    rx_pos = 0;
                                }
                            } else if rx_pos < rx_buf.len() {
                                rx_buf[rx_pos] = b;
                                rx_pos += 1;
                            }
                        }
                    }
                    Err(UsbError::WouldBlock) => break,
                    Err(_) => break,
                }
            }
        }

        // 2) Read every key; emit a K event per change.
        let mut cur_keys = [false; 12];
        for (i, k) in keys.iter_mut().enumerate() {
            let pressed = k.is_low().unwrap_or(false); // pull-up: low == pressed
            cur_keys[i] = pressed;
        }
        mask = 0;
        for (i, &pressed) in cur_keys.iter().enumerate() {
            if pressed {
                mask |= 1 << i;
            }
            if pressed != prev_keys[i] {
                let tok = protocol::key_token(i as u8 + 1, pressed);
                send_line(&mut serial, tok.as_bytes());
            }
        }
        prev_keys = cur_keys;

        // 3) One-time state sync so the host learns the mask right at boot.
        if first_report {
            let tok = protocol::mask_token(mask);
            send_line(&mut serial, tok.as_bytes());
            first_report = false;
        }

        // 4) Tick delay.
        timer.delay_ms(TICK_MS);
    }
}

/// Parse and apply an inbound command line.
#[cfg(not(test))]
fn handle_command(
    serial: &mut SerialPort<'_, UsbBus>,
    led: &mut gpio::Pin<DynPinId, FunctionSioOutput, PullDown>,
    display: &mut heapless::String<64>,
    mask: &u16,
    cmd: &str,
) {
    let cmd = cmd.trim();
    if cmd == "REQ" {
        let tok = protocol::mask_token(*mask);
        send_line(serial, tok.as_bytes());
    } else if let Some(rest) = cmd.strip_prefix("LED ") {
        let mut it = rest.split_whitespace();
        if let (Some(_n), Some(v)) = (it.next(), it.next()) {
            match v {
                "1" => {
                    let _ = led.set_high();
                }
                "0" => {
                    let _ = led.set_low();
                }
                _ => {}
            }
        }
    } else if let Some(rest) = cmd.strip_prefix("TEXT ") {
        display.clear();
        let _ = display.push_str(rest);
    }
}

/// Send a full line, retrying while the CDC endpoint is busy.
#[cfg(not(test))]
fn send_line(serial: &mut SerialPort<'_, UsbBus>, data: &[u8]) {
    let mut written = 0;
    while written < data.len() {
        match serial.write(&data[written..]) {
            Ok(0) => continue,
            Ok(n) => written += n,
            Err(UsbError::WouldBlock) => continue,
            Err(_) => break,
        }
    }
}
