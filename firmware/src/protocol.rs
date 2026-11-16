//! Compact serial event protocol for the macropad.
//!
//! One line per event, ASCII, newline-terminated (host sees /dev/ttyACMx):
//!
//!   K<NN><0|1>   key NN (01..=12) down(1)/up(0)   e.g. "K031"  = key 3 down
//!   E+ / E-      encoder one detent CW / CCW
//!   S0 / S1      encoder push switch (BOOT button) up / down
//!   R<hhh>       reply to REQ — current 12-key mask, 3 lowercase hex digits
//!
//!   PONG         reply to the host's PING heartbeat
//!
//! Host -> board commands are parsed by [`parse_command`] (see docs/PROTOCOL.md).
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

/// "PONG\n" — heartbeat reply.
pub fn pong_token() -> String<8> {
    let mut s = String::new();
    let _ = s.push_str("PONG\n");
    s
}

/// Longest key label the OLED cell can show (7 chars * 6 px = 42 px).
pub const LABEL_MAX: usize = 7;

/// A parsed host -> board command.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Ping,
    Req,
    /// Status LED (legacy): index, on/off.
    Led(u8, bool),
    /// Key colour: 0-based key index, RGB.
    Color(u8, [u8; 3]),
    /// Key label: 0-based key index, text (printable ASCII, <= LABEL_MAX).
    Label(u8, String<8>),
    /// Volume 0..=100.
    Volume(u8),
    Text(String<64>),
}

fn parse_key(tok: &str) -> Option<u8> {
    let n: u8 = tok.parse().ok()?;
    if (1..=12).contains(&n) {
        Some(n - 1)
    } else {
        None
    }
}

fn parse_hex_rgb(tok: &str) -> Option<[u8; 3]> {
    if tok.len() != 6 || !tok.is_ascii() {
        return None;
    }
    let v = u32::from_str_radix(tok, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

/// Parse one command line (without the newline). `None` = unknown/malformed.
pub fn parse_command(line: &str) -> Option<Command> {
    let line = line.trim();
    if line == "PING" {
        return Some(Command::Ping);
    }
    if line == "REQ" {
        return Some(Command::Req);
    }
    if let Some(rest) = line.strip_prefix("LED ") {
        let mut it = rest.split_whitespace();
        let n: u8 = it.next()?.parse().ok()?;
        return match it.next()? {
            "1" => Some(Command::Led(n, true)),
            "0" => Some(Command::Led(n, false)),
            _ => None,
        };
    }
    if let Some(rest) = line.strip_prefix("COL ") {
        let mut it = rest.split_whitespace();
        let key = parse_key(it.next()?)?;
        let rgb = parse_hex_rgb(it.next()?)?;
        return Some(Command::Color(key, rgb));
    }
    if let Some(rest) = line.strip_prefix("LBL ") {
        let (key_tok, text) = match rest.split_once(' ') {
            Some((k, t)) => (k, t),
            None => (rest, ""),
        };
        let key = parse_key(key_tok)?;
        let mut label: String<8> = String::new();
        for ch in text.chars().take(LABEL_MAX) {
            let _ = label.push(if (' '..='~').contains(&ch) { ch } else { '?' });
        }
        return Some(Command::Label(key, label));
    }
    if let Some(rest) = line.strip_prefix("VOL ") {
        let v: u32 = rest.trim().parse().ok()?;
        return Some(Command::Volume(v.min(100) as u8));
    }
    if let Some(rest) = line.strip_prefix("TEXT ") {
        let mut t: String<64> = String::new();
        let _ = t.push_str(rest);
        return Some(Command::Text(t));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pong() {
        assert_eq!(pong_token().as_str(), "PONG\n");
    }

    #[test]
    fn parse_ping_req() {
        assert_eq!(parse_command("PING"), Some(Command::Ping));
        assert_eq!(parse_command("REQ\r"), Some(Command::Req));
    }

    #[test]
    fn parse_color() {
        assert_eq!(
            parse_command("COL 03 ff8000"),
            Some(Command::Color(2, [0xff, 0x80, 0x00]))
        );
        assert_eq!(parse_command("COL 12 00000a"), Some(Command::Color(11, [0, 0, 10])));
    }

    #[test]
    fn parse_color_rejects_bad_input() {
        assert_eq!(parse_command("COL 00 ffffff"), None);
        assert_eq!(parse_command("COL 13 ffffff"), None);
        assert_eq!(parse_command("COL 01 fff"), None);
        assert_eq!(parse_command("COL 01 gggggg"), None);
        assert_eq!(parse_command("COL 01"), None);
    }

    #[test]
    fn parse_label() {
        match parse_command("LBL 01 Airhorn") {
            Some(Command::Label(0, l)) => assert_eq!(l.as_str(), "Airhorn"),
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn parse_label_truncates_and_keeps_spaces() {
        match parse_command("LBL 02 Sad trombone") {
            Some(Command::Label(1, l)) => assert_eq!(l.as_str(), "Sad tro"),
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn parse_label_empty_and_nonascii() {
        match parse_command("LBL 05") {
            Some(Command::Label(4, l)) => assert_eq!(l.as_str(), ""),
            other => panic!("{:?}", other),
        }
        match parse_command("LBL 06 h\u{e9}llo") {
            Some(Command::Label(5, l)) => assert_eq!(l.as_str(), "h?llo"),
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn parse_volume_clamps() {
        assert_eq!(parse_command("VOL 42"), Some(Command::Volume(42)));
        assert_eq!(parse_command("VOL 250"), Some(Command::Volume(100)));
        assert_eq!(parse_command("VOL 120"), Some(Command::Volume(100)));
        assert_eq!(parse_command("VOL x"), None);
    }

    #[test]
    fn parse_led_legacy() {
        assert_eq!(parse_command("LED 0 1"), Some(Command::Led(0, true)));
        assert_eq!(parse_command("LED 0 2"), None);
    }

    #[test]
    fn parse_unknown() {
        assert_eq!(parse_command("HELLO"), None);
        assert_eq!(parse_command(""), None);
    }

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
