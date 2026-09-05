//! Compact serial event protocol for the macropad.
//!
//! One line per event, ASCII, newline-terminated (host sees /dev/ttyACMx):
//!
//!   K<NN><0|1>   key NN (01..=12) down(1)/up(0)   e.g. "K031"  = key 3 down
//!   E+ / E-      encoder one detent CW / CCW
//!   S0 / S1      encoder push switch (BOOT button) up / down
//!   R<hhh>       reply to REQ — current 12-key mask, 3 lowercase hex digits
//!
//! Fixed-width tokens (no spaces) so a line-oriented parser on the host can
//! split on '\n' and match exact prefixes.

use core::fmt::Write;
use heapless::String;

/// "K<NN><0|1>\n" — key press/release event.
pub fn key_token(idx: u8, down: bool) -> String<8> {
    let mut s = String::new();
    let _ = write!(s, "K{:02}{}\n", idx, if down { 1 } else { 0 });
    s
}

/// "E+\n" / "E-\n" — one encoder detent.
pub fn encoder_token(dir: i8) -> String<8> {
    let mut s = String::new();
    let _ = write!(s, "E{}\n", if dir > 0 { '+' } else { '-' });
    s
}

/// "S0\n" / "S1\n" — encoder push switch state.
pub fn switch_token(down: bool) -> String<8> {
    let mut s = String::new();
    let _ = write!(s, "S{}\n", if down { 1 } else { 0 });
    s
}

/// "R<hhh>\n" — current 12-key mask as 3 lowercase hex digits.
pub fn mask_token(mask: u16) -> String<8> {
    let mut s = String::new();
    let _ = write!(s, "R{:03x}\n", mask);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_down_token() {
        assert_eq!(key_token(3, true).as_str(), "K031\n");
    }

    #[test]
    fn key_up_token() {
        assert_eq!(key_token(12, false).as_str(), "K120\n");
    }

    #[test]
    fn key_single_digit_is_zero_padded() {
        assert_eq!(key_token(1, true).as_str(), "K011\n");
    }

    #[test]
    fn encoder_cw_token() {
        assert_eq!(encoder_token(1).as_str(), "E+\n");
    }

    #[test]
    fn encoder_ccw_token() {
        assert_eq!(encoder_token(-1).as_str(), "E-\n");
    }

    #[test]
    fn switch_down_token() {
        assert_eq!(switch_token(true).as_str(), "S1\n");
    }

    #[test]
    fn switch_up_token() {
        assert_eq!(switch_token(false).as_str(), "S0\n");
    }

    #[test]
    fn mask_token_lowercase_hex() {
        assert_eq!(mask_token(0x0a7).as_str(), "R0a7\n");
    }

    #[test]
    fn mask_token_full_mask() {
        assert_eq!(mask_token(0xfff).as_str(), "Rfff\n");
    }

    #[test]
    fn mask_token_empty() {
        assert_eq!(mask_token(0).as_str(), "R000\n");
    }
}
