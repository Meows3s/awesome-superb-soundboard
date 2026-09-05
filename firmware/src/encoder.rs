//! Rotary encoder decoding + input debouncing (pure logic, no hardware).
//!
//! `QuadDecoder` converts raw A/B quadrature samples into one tick per detent
//! (the macropad encoder has 20 detents/rev, 4 edges per detent). `Debouncer`
//! requires N consecutive identical samples before a state change registers,
//! so mechanical bounce and 1-tick jitter never produce spurious events.

use core::cmp::Ordering;

/// Quadrature decoder: feed A/B samples, get one +1/-1 per detent.
///
/// Tracks a phase accumulator: each valid edge adds +1 (CW) or -1 (CCW); a
/// detent completes when the phase reaches +4/-4. Back-and-forth jitter
/// cancels because the phase returns toward zero. The `state` field is a
/// 2-bit (A,B) snapshot; `0xff` is the "not yet sampled" sentinel.
#[derive(Clone, Copy)]
pub struct QuadDecoder {
    state: u8,
    phase: i8,
}

impl QuadDecoder {
    pub const fn new() -> Self {
        Self {
            state: 0xff,
            phase: 0,
        }
    }

    /// Feed one sample pair. Returns +1 for a full CW detent, -1 for a full
    /// CCW detent, 0 otherwise (including bounces and invalid jumps).
    pub fn update(&mut self, a: bool, b: bool) -> i8 {
        let new = (a as u8) << 1 | (b as u8);
        if self.state != 0xff {
            let delta = match (self.state, new) {
                // CW: 00 -> 01 -> 11 -> 10 -> 00
                (0, 1) | (1, 3) | (3, 2) | (2, 0) => 1,
                // CCW: 00 -> 10 -> 11 -> 01 -> 00
                (0, 2) | (2, 3) | (3, 1) | (1, 0) => -1,
                // Invalid jump or a bounce that skipped an edge: track the new
                // state but contribute nothing.
                _ => 0,
            };
            self.phase += delta;
            match self.phase.cmp(&4) {
                Ordering::Greater | Ordering::Equal => {
                    self.phase = 0;
                    self.state = new;
                    return 1;
                }
                _ => {}
            }
            match self.phase.cmp(&-4) {
                Ordering::Less | Ordering::Equal => {
                    self.phase = 0;
                    self.state = new;
                    return -1;
                }
                _ => {}
            }
        }
        self.state = new;
        0
    }
}

/// Debouncer: requires `needed` consecutive samples equal to the candidate
/// before the reported state flips. Any opposing sample resets the count.
#[derive(Clone, Copy)]
pub struct Debouncer {
    stable: u8,
    state: bool,
}

impl Debouncer {
    pub const fn new() -> Self {
        Self {
            stable: 0,
            state: false,
        }
    }

    /// Feed one raw sample; returns the current debounced state.
    pub fn update(&mut self, sample: bool, needed: u8) -> bool {
        if sample != self.state {
            self.stable = self.stable.saturating_add(1);
            if self.stable >= needed {
                self.state = sample;
                self.stable = 0;
            }
        } else {
            self.stable = 0;
        }
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (a, b) from a 2-bit state where a is the high bit.
    fn s(state: u8) -> (bool, bool) {
        (state & 2 != 0, state & 1 != 0)
    }

    // --- QuadDecoder --------------------------------------------------------

    #[test]
    fn cw_detent_ticks_once_per_full_rotation() {
        let mut q = QuadDecoder::new();
        // 00 -> 01 -> 11 -> 10 -> 00 is one CW detent (4 edges).
        let states = [0u8, 1, 3, 2, 0];
        let mut ticks = Vec::new();
        for st in states {
            let (a, b) = s(st);
            ticks.push(q.update(a, b));
        }
        assert_eq!(ticks, vec![0, 0, 0, 0, 1]);
    }

    #[test]
    fn ccw_detent_ticks_negative() {
        let mut q = QuadDecoder::new();
        // 00 -> 10 -> 11 -> 01 -> 00 is one CCW detent.
        let states = [0u8, 2, 3, 1, 0];
        let mut ticks = Vec::new();
        for st in states {
            let (a, b) = s(st);
            ticks.push(q.update(a, b));
        }
        assert_eq!(ticks, vec![0, 0, 0, 0, -1]);
    }

    #[test]
    fn edge_jitter_cancels() {
        let mut q = QuadDecoder::new();
        // Back-and-forth on one edge never completes a detent.
        let states = [0u8, 1, 0, 1, 0, 1, 0];
        for st in states {
            let (a, b) = s(st);
            assert_eq!(q.update(a, b), 0);
        }
    }

    #[test]
    fn partial_then_reverse_cancels() {
        let mut q = QuadDecoder::new();
        // 3 edges CW (phase 3) then 3 edges CCW back to start: no tick.
        for st in [0u8, 1, 3, 2, 3, 1, 0] {
            let (a, b) = s(st);
            assert_eq!(q.update(a, b), 0);
        }
    }

    #[test]
    fn repeated_rotations_accumulate() {
        let mut q = QuadDecoder::new();
        let mut ticks = Vec::new();
        for st in [0u8, 1, 3, 2, 0, 1, 3, 2, 0] {
            let (a, b) = s(st);
            ticks.push(q.update(a, b));
        }
        assert_eq!(ticks, vec![0, 0, 0, 0, 1, 0, 0, 0, 1]);
    }

    #[test]
    fn invalid_state_jump_ignored() {
        let mut q = QuadDecoder::new();
        let (a0, b0) = s(0);
        q.update(a0, b0);
        // 00 -> 11 is not a valid quadrature transition: no tick, no crash.
        let (a, b) = s(3);
        assert_eq!(q.update(a, b), 0);
    }

    // --- Debouncer ----------------------------------------------------------

    #[test]
    fn needs_n_consecutive_samples() {
        let mut d = Debouncer::new();
        for _ in 0..4 {
            assert!(!d.update(true, 5));
        }
        assert!(d.update(true, 5));
    }

    #[test]
    fn jitter_never_flips_state() {
        let mut d = Debouncer::new();
        let samples = [true, false, true, false, true, false, true, false];
        for smp in samples {
            assert!(!d.update(smp, 5));
        }
    }

    #[test]
    fn flips_back_after_settling() {
        let mut d = Debouncer::new();
        assert!(d.update(true, 1));
        for _ in 0..2 {
            assert!(d.update(false, 3));
        }
        assert!(!d.update(false, 3));
    }

    #[test]
    fn needed_one_flips_immediately() {
        let mut d = Debouncer::new();
        assert!(d.update(true, 1));
        assert!(!d.update(false, 1));
    }

    #[test]
    fn a_single_opposing_sample_resets_the_count() {
        let mut d = Debouncer::new();
        assert!(!d.update(true, 3)); // 1
        assert!(!d.update(true, 3)); // 2
        assert!(!d.update(false, 3)); // interrupts
        assert!(!d.update(true, 3)); // 1 again
        assert!(!d.update(true, 3)); // 2
        assert!(d.update(true, 3)); // 3 -> flip
    }

    #[test]
    fn array_init_works_and_starts_false() {
        let arr = [Debouncer::new(); 12];
        for d in arr {
            assert!(!d.state);
        }
    }
}
