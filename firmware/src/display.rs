//! SH1106 128x64 OLED driver (Adafruit Macropad: SPI1, CS=GPIO22, DC=GPIO24,
//! RST=GPIO23) plus the soundboard screen layout.
//!
//! The UI is rendered into a 1 KiB framebuffer with embedded-graphics and
//! pushed page-by-page. Only compiled for the device build.

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::SpiBus;
use heapless::String;

pub const WIDTH: usize = 128;
pub const HEIGHT: usize = 64;
const PAGES: usize = HEIGHT / 8;
/// SH1106 RAM is 132 columns wide; a 128-px panel is centred in it.
const COLUMN_OFFSET: u8 = 2;
/// Set to flip the picture 180 degrees if the text shows up upside down.
const ROTATE_180: bool = false;

const CELL_W: i32 = 43; // 3 columns of 42-px labels (7 chars * 6 px)
const CELL_H: i32 = 13;
const GRID_TOP: i32 = 13;

/// What the screen should show. `render` is a pure function of this.
pub struct Screen<'a> {
    pub connected: bool,
    pub volume: u8,
    pub labels: &'a [String<8>; 12],
    pub pressed: u16,
}

struct FrameBuffer {
    buf: [u8; WIDTH * PAGES],
}

impl FrameBuffer {
    fn clear(&mut self) {
        self.buf = [0; WIDTH * PAGES];
    }
}

impl OriginDimensions for FrameBuffer {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

impl DrawTarget for FrameBuffer {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<BinaryColor>>,
    {
        for Pixel(p, c) in pixels {
            if p.x < 0 || p.y < 0 || p.x >= WIDTH as i32 || p.y >= HEIGHT as i32 {
                continue;
            }
            let (x, y) = (p.x as usize, p.y as usize);
            let idx = (y / 8) * WIDTH + x;
            let bit = 1u8 << (y % 8);
            if c == BinaryColor::On {
                self.buf[idx] |= bit;
            } else {
                self.buf[idx] &= !bit;
            }
        }
        Ok(())
    }
}

pub struct Oled<SPI, DC, CS, RST> {
    spi: SPI,
    dc: DC,
    cs: CS,
    rst: RST,
    fb: FrameBuffer,
}

impl<SPI: SpiBus, DC: OutputPin, CS: OutputPin, RST: OutputPin> Oled<SPI, DC, CS, RST> {
    pub fn new(spi: SPI, dc: DC, cs: CS, rst: RST) -> Self {
        Self {
            spi,
            dc,
            cs,
            rst,
            fb: FrameBuffer { buf: [0; WIDTH * PAGES] },
        }
    }

    fn command(&mut self, bytes: &[u8]) {
        let _ = self.dc.set_low();
        let _ = self.cs.set_low();
        let _ = self.spi.write(bytes);
        let _ = self.spi.flush();
        let _ = self.cs.set_high();
    }

    /// Hardware reset + init. `delay_ms` is the caller's delay source.
    pub fn init(&mut self, mut delay_ms: impl FnMut(u32)) {
        let _ = self.cs.set_high();
        let _ = self.rst.set_high();
        delay_ms(5);
        let _ = self.rst.set_low();
        delay_ms(10);
        let _ = self.rst.set_high();
        delay_ms(10);
        let (seg, com) = if ROTATE_180 { (0xA1, 0xC8) } else { (0xA0, 0xC0) };
        self.command(&[
            0xAE, // display off
            0xD5, 0x80, // clock divide
            0xA8, 0x3F, // multiplex 1/64
            0xD3, 0x00, // display offset
            0x40, // start line 0
            0xAD, 0x8B, // internal DC-DC on
            seg, com, // orientation
            0xDA, 0x12, // COM pins
            0x81, 0x80, // contrast
            0xD9, 0x22, // pre-charge
            0xDB, 0x35, // VCOM deselect
            0xA4, 0xA6, // follow RAM, normal (non-inverted)
            0xAF, // display on
        ]);
        self.fb.clear();
        self.flush();
    }

    /// Push the framebuffer to the panel.
    fn flush(&mut self) {
        for page in 0..PAGES {
            self.command(&[
                0xB0 | page as u8,
                COLUMN_OFFSET & 0x0F,
                0x10 | (COLUMN_OFFSET >> 4),
            ]);
            let _ = self.dc.set_high();
            let _ = self.cs.set_low();
            let start = page * WIDTH;
            let _ = self.spi.write(&self.fb.buf[start..start + WIDTH]);
            let _ = self.spi.flush();
            let _ = self.cs.set_high();
        }
    }

    pub fn draw(&mut self, screen: &Screen) {
        render(&mut self.fb, screen);
        self.flush();
    }
}

fn render(fb: &mut FrameBuffer, s: &Screen) {
    fb.clear();
    let on = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let off = MonoTextStyle::new(&FONT_6X10, BinaryColor::Off);
    let fill = PrimitiveStyle::with_fill(BinaryColor::On);

    // Header bar: inverted + loud when there is no host, so it can't be missed.
    if s.connected {
        let mut head: String<24> = String::new();
        let _ = core::fmt::write(&mut head, format_args!("CONNECTED  VOL {:>3}", s.volume));
        let _ = Text::with_baseline(&head, Point::new(0, 1), on, Baseline::Top).draw(fb);
        let _ = Rectangle::new(Point::new(0, 11), Size::new(WIDTH as u32, 1))
            .into_styled(fill)
            .draw(fb);
    } else {
        let _ = Rectangle::new(Point::zero(), Size::new(WIDTH as u32, 12))
            .into_styled(fill)
            .draw(fb);
        let _ = Text::with_baseline("NOT CONNECTED", Point::new(0, 1), off, Baseline::Top).draw(fb);
    }

    for i in 0..12usize {
        let (col, row) = ((i % 3) as i32, (i / 3) as i32);
        let (x, y) = (col * CELL_W, GRID_TOP + row * CELL_H);
        let down = s.pressed & (1 << i) != 0;
        let (style, text) = if down { (off, true) } else { (on, false) };
        if text {
            let _ = Rectangle::new(Point::new(x, y), Size::new(42, 12))
                .into_styled(fill)
                .draw(fb);
        }
        let label = s.labels[i].as_str();
        let shown = if label.is_empty() { "-" } else { label };
        let w = shown.len() as i32 * 6;
        let _ = Text::with_baseline(shown, Point::new(x + (42 - w) / 2, y + 1), style, Baseline::Top)
            .draw(fb);
    }
}
