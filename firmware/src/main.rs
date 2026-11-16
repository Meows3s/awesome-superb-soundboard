//! awesome-superb-soundboard firmware for the Adafruit RP2040 Macropad.
//!
//! Synchronous, no-async design so it builds on current stable Rust. Each tick
//! (2 ms) it reads the keys, rotary encoder, and encoder push switch, and
//! reports events over USB-CDC serial (the same USB cable; host sees
//! /dev/ttyACMx). It also listens for incoming commands to drive the status
//! LED and a display text buffer.
//!
//! Pinout (from the `adafruit-macropad` BSP):
//!   enc switch (BOOT button) = GPIO0
//!   key1..12 = GPIO1..GPIO12   status LED = GPIO13   speaker = GPIO16
//!   enc_a = GPIO17  enc_b = GPIO18  neopixel = GPIO19
//!   sda = GPIO20  scl = GPIO21  oled_cs = GPIO22  oled_reset = GPIO23
//!   oled_dc = GPIO24  spi_sclk = GPIO26  spi_mosi = GPIO27  spi_miso = GPIO28
//!
//! Serial protocol: see docs/PROTOCOL.md (ASCII, newline-terminated).
//!   board -> host: K<NN><d>, E+/E-, S0/S1, R<hhh>, PONG
//!   host -> board: PING, COL, LBL, VOL, REQ, LED, TEXT
//! The board treats the host as "connected" while PINGs arrive (<2 s apart);
//! otherwise the OLED shows a NOT CONNECTED banner.
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

#[cfg(not(test))]
mod display;
mod encoder;
mod protocol;

#[cfg(not(test))]
use crate::encoder::{Debouncer, QuadDecoder};
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
    fugit::RateExtU32,
    gpio::{self, DynPinId, FunctionPio0, FunctionSioInput, FunctionSpi, PullUp},
    pac,
    pio::PIOExt,
    sio::Sio,
    spi::Spi,
    usb::UsbBus,
    Clock, Timer,
};
#[cfg(not(test))]
use smart_leds_trait::{SmartLedsWrite, RGB8};
#[cfg(not(test))]
use ws2812_pio::Ws2812Direct;
#[cfg(not(test))]
use usb_device::class_prelude::UsbBusAllocator;
#[cfg(not(test))]
use usb_device::device::{StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbVidPid};
#[cfg(not(test))]
use usb_device::prelude::UsbError;
#[cfg(not(test))]
use usbd_serial::{SerialPort, USB_CLASS_CDC};

/// Second-stage bootloader (configures the QSPI flash for XIP). The macropad
/// uses a W25Q64JV; the W25Q080 routine is the generic compatible choice.
#[cfg(not(test))]
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

/// The macropad crystal is 12 MHz (same as the Pico).
#[cfg(not(test))]
const XOSC_CRYSTAL_FREQ: u32 = 12_000_000;
/// Tick period. 2 ms keeps up with fast encoder turns (20 detents/rev; a
/// detent is 4 edges, so 5 rev/s = 400 edges/s = one edge per 2.5 ms).
#[cfg(not(test))]
const TICK_MS: u32 = 2;
/// Consecutive identical samples required before a key state change registers
/// (2 ms tick => 10 ms debounce window).
#[cfg(not(test))]
const KEY_DEBOUNCE: u8 = 5;
/// Consecutive identical samples required before the encoder switch flips.
#[cfg(not(test))]
const SW_DEBOUNCE: u8 = 5;

/// Host is considered gone when no PING has arrived for this long.
#[cfg(not(test))]
const HOST_TIMEOUT_US: u64 = 2_000_000;
/// NeoPixel brightness (out of 255) of an idle key vs. a pressed key.
#[cfg(not(test))]
const LED_IDLE: u16 = 40;
#[cfg(not(test))]
const LED_PRESSED: u16 = 255;
/// Give up writing to a stalled USB endpoint after this many spins, so a
/// host that is not reading can never hang the key/encoder scan loop.
#[cfg(not(test))]
const SEND_SPIN_LIMIT: u32 = 20_000;

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

    // Encoder A/B, encoder push switch (BOOT button), and the status LED.
    // (NeoPixel sense pin dropped for now — NeoPixel feedback is a later task.)
    let mut enc_a = pins.gpio17.into_pull_up_input().into_dyn_pin();
    let mut enc_b = pins.gpio18.into_pull_up_input().into_dyn_pin();
    let mut sw_pin = pins.gpio0.into_pull_up_input().into_dyn_pin();
    let mut led = pins.gpio13.into_push_pull_output().into_dyn_pin();

    let mut timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    // --- NeoPixels: 12 WS2812 on GPIO19 via PIO0 ----------------------------
    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let mut pixels = Ws2812Direct::new(
        pins.gpio19.into_function::<FunctionPio0>(),
        &mut pio,
        sm0,
        clocks.peripheral_clock.freq(),
    );

    // --- OLED: SH1106 on SPI1 (SCLK=26, MOSI=27), CS=22, DC=24, RST=23 -------
    let spi_mosi = pins.gpio27.into_function::<FunctionSpi>();
    let spi_sclk = pins.gpio26.into_function::<FunctionSpi>();
    let spi = Spi::<_, _, _, 8>::new(pac.SPI1, (spi_mosi, spi_sclk)).init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        8.MHz(),
        embedded_hal::spi::MODE_0,
    );
    let mut oled = display::Oled::new(
        spi,
        pins.gpio24.into_push_pull_output(),
        pins.gpio22.into_push_pull_output(),
        pins.gpio23.into_push_pull_output(),
    );
    oled.init(|ms| timer.delay_ms(ms));

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
        UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x2e8a, 0x000a))
            // Windows' usbser driver needs the device-level CDC class (Linux
            // is lenient); without it the COM port appears but cannot be opened.
            .strings(&[StringDescriptors::default()
                .manufacturer("a.s.s.")
                .product("Soundboard Macropad")
                .serial_number("0001")])
            .unwrap()
            .device_class(USB_CLASS_CDC)
            .build();

    let mut labels: [heapless::String<8>; 12] = Default::default();
    let mut colors: [[u8; 3]; 12] = [[0, 0, 0]; 12];
    let mut volume: u8 = 0;
    // Timestamp (us since boot) of the last PING; None = never / timed out.
    let mut last_ping_us: Option<u64> = None;
    let mut connected = false;
    let mut dirty = true; // OLED / LED redraw needed
    let mut key_deb: [Debouncer; 12] = [Debouncer::new(); 12];
    let mut prev_keys: [bool; 12] = [false; 12];
    let mut quad = QuadDecoder::new();
    let mut sw_deb = Debouncer::new();
    let mut prev_sw = false;
    let mut mask: u16 = 0;
    let mut first_report = true;
    let mut shown_mask: u16 = 0;
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
                                        match protocol::parse_command(s) {
                                            Some(protocol::Command::Ping) => {
                                                last_ping_us = Some(timer.get_counter().ticks());
                                                let tok = protocol::pong_token();
                                                send_line(&mut serial, tok.as_bytes());
                                                if !connected {
                                                    connected = true;
                                                    dirty = true;
                                                }
                                            }
                                            Some(protocol::Command::Req) => {
                                                let tok = protocol::mask_token(mask);
                                                send_line(&mut serial, tok.as_bytes());
                                            }
                                            Some(protocol::Command::Led(_, on)) => {
                                                let _ = if on { led.set_high() } else { led.set_low() };
                                            }
                                            Some(protocol::Command::Color(k, rgb)) => {
                                                colors[k as usize] = rgb;
                                                dirty = true;
                                            }
                                            Some(protocol::Command::Label(k, text)) => {
                                                labels[k as usize] = text;
                                                dirty = true;
                                            }
                                            Some(protocol::Command::Volume(v)) => {
                                                volume = v;
                                                dirty = true;
                                            }
                                            Some(protocol::Command::Text(_)) => {} // legacy, unused
                                            None => {}
                                        }
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

        // 2) Read every key; debounce; emit a K event per debounced change.
        let mut cur_keys = [false; 12];
        for (i, k) in keys.iter_mut().enumerate() {
            let pressed = k.is_low().unwrap_or(false); // pull-up: low == pressed
            cur_keys[i] = pressed;
        }
        mask = 0;
        let mut debounced_keys = [false; 12];
        for i in 0..12 {
            let debounced = key_deb[i].update(cur_keys[i], KEY_DEBOUNCE);
            debounced_keys[i] = debounced;
            if debounced {
                mask |= 1 << i;
            }
            if debounced != prev_keys[i] {
                let tok = protocol::key_token(i as u8 + 1, debounced);
                send_line(&mut serial, tok.as_bytes());
            }
        }
        prev_keys = debounced_keys;

        // 3) Encoder: one E+ / E- per detent.
        let tick = quad.update(
            enc_a.is_low().unwrap_or(false),
            enc_b.is_low().unwrap_or(false),
        );
        if tick > 0 {
            let tok = protocol::encoder_token(1);
            send_line(&mut serial, tok.as_bytes());
        } else if tick < 0 {
            let tok = protocol::encoder_token(-1);
            send_line(&mut serial, tok.as_bytes());
        }

        // 4) Encoder push switch: S0 / S1 per debounced change.
        let sw_state = sw_deb.update(sw_pin.is_low().unwrap_or(false), SW_DEBOUNCE);
        if sw_state != prev_sw {
            let tok = protocol::switch_token(sw_state);
            send_line(&mut serial, tok.as_bytes());
            prev_sw = sw_state;
        }

        // 5) One-time state sync so the host learns the mask right at boot.
        if first_report {
            let tok = protocol::mask_token(mask);
            send_line(&mut serial, tok.as_bytes());
            first_report = false;
        }

        // 6) Heartbeat timeout: drop to NOT CONNECTED if the host went quiet.
        let now_us = timer.get_counter().ticks();
        if connected {
            let alive = matches!(last_ping_us, Some(t) if now_us.wrapping_sub(t) < HOST_TIMEOUT_US);
            if !alive {
                connected = false;
                dirty = true;
            }
        }

        // 7) Refresh LEDs + OLED when something changed (key press or command).
        if mask != shown_mask {
            shown_mask = mask;
            dirty = true;
        }
        if dirty {
            dirty = false;
            let frame = (0..12).map(|i| {
                let level = if mask & (1 << i) != 0 { LED_PRESSED } else { LED_IDLE };
                let [r, g, b] = colors[i];
                RGB8::new(
                    (r as u16 * level / 255) as u8,
                    (g as u16 * level / 255) as u8,
                    (b as u16 * level / 255) as u8,
                )
            });
            let _ = pixels.write(frame);
            oled.draw(&display::Screen {
                connected,
                volume,
                labels: &labels,
                pressed: mask,
            });
        }

        // 8) Tick delay.
        timer.delay_ms(TICK_MS);
    }
}

/// Send a full line. Bounded: if the host is not draining the endpoint (no
/// app open, or the port is closed) we drop the line rather than block the
/// scan loop forever.
#[cfg(not(test))]
fn send_line(serial: &mut SerialPort<'_, UsbBus>, data: &[u8]) {
    let mut written = 0;
    let mut spins = 0u32;
    while written < data.len() {
        match serial.write(&data[written..]) {
            Ok(0) | Err(UsbError::WouldBlock) => {
                spins += 1;
                if spins > SEND_SPIN_LIMIT {
                    break;
                }
            }
            Ok(n) => written += n,
            Err(_) => break,
        }
    }
}
