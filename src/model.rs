use crate::arithmetic::{PROB_BITS, PROB_MAX, PROB_MIN};

const STATE_BITS: u32 = 16;
const STATE_SCALE: u32 = 1 << STATE_BITS;

const _: () = assert!(
    STATE_BITS >= PROB_BITS,
    "STATE_BITS must be atleast PROB_BITS"
);

const LEARNING_RATE: u32 = 4;

const CONTEXTS: usize = 1 << u8::BITS;

pub struct Order0 {
    probs: [u32; CONTEXTS],
    context: usize,
}

impl Order0 {
    pub fn new() -> Self {
        Order0 {
            probs: [STATE_SCALE / 2; CONTEXTS],
            context: 1,
        }
    }

    /// Probability that the next bit is 1, in the coder's units.
    pub fn predict(&self) -> u32 {
        let p = self.probs[self.context] >> (STATE_BITS - PROB_BITS);
        p.clamp(PROB_MIN, PROB_MAX)
    }

    /// Learn from the bit that really came, then move to the next context.
    pub fn update(&mut self, bit: bool) {
        let p = &mut self.probs[self.context];
        if bit {
            *p += (STATE_SCALE - *p) >> LEARNING_RATE;
        } else {
            *p -= *p >> LEARNING_RATE;
        }

        self.context = self.context * 2 + bit as usize;
        if self.context >= CONTEXTS {
            self.context = 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arithmetic::PROB_SCALE;

    #[test]
    fn starts_at_fifty_percent() {
        assert_eq!(Order0::new().predict(), PROB_SCALE / 2);
    }

    #[test]
    fn learns_from_repeated_bytes() {
        // 100 times the byte 0xFF: every bit is 1
        let mut model = Order0::new();
        for _ in 0..100 {
            for _ in 0..u8::BITS {
                model.update(true);
            }
        }
        assert!(
            model.predict() > PROB_SCALE * 9 / 10,
            "p = {}",
            model.predict()
        );
    }

    #[test]
    fn prediction_stays_in_valid_range() {
        let mut model = Order0::new();
        for i in 0..10_000 {
            model.update(i % 3 == 0);
            assert!((PROB_MIN..=PROB_MAX).contains(&model.predict()));
        }
        // Only zeros: p goes very close to 0, but must never reach it
        let mut model = Order0::new();
        for _ in 0..10_000 {
            model.update(false);
            assert!((PROB_MIN..=PROB_MAX).contains(&model.predict()));
        }
    }
}
