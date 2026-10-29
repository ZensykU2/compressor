use crate::arithmetic::{PROB_BITS, PROB_MAX, PROB_MIN};

const STATE_BITS: u32 = 16;
const STATE_SCALE: u32 = 1 << STATE_BITS;

const _: () = assert!(
    STATE_BITS >= PROB_BITS,
    "STATE_BITS must be at least PROB_BITS"
);

/// In the long run a counter moves 1/2^LEARNING_RATE towards each new bit.
const LEARNING_RATE: u32 = 4;

/// A counter learns with weight 1/(count + 2). Once count reaches COUNT_LIMIT,
/// the weight stays at 1/(COUNT_LIMIT + 2) = 1/2^LEARNING_RATE.
const COUNT_LIMIT: u8 = ((1u32 << LEARNING_RATE) - 2) as u8;

const _: () = assert!(
    (1u32 << LEARNING_RATE) - 2 <= u8::MAX as u32,
    "COUNT_LIMIT must fit into a u8"
);

/// A table of adaptive probabilities. Every slot learns fast at first, then settles.
pub struct CounterTable {
    probs: Vec<u32>,
    counts: Vec<u8>, // how often each slot has seen a bit
}

impl CounterTable {
    pub fn new(size: usize) -> Self {
        CounterTable {
            probs: vec![STATE_SCALE / 2; size],
            counts: vec![0; size],
        }
    }

    /// Probability that the next bit is 1 in slot `i`, in the coder's units.
    pub fn predict(&self, i: usize) -> u32 {
        let p = self.probs[i] >> (STATE_BITS - PROB_BITS);
        p.clamp(PROB_MIN, PROB_MAX)
    }

    /// Slot `i` learns from the bit that really came.
    pub fn update(&mut self, i: usize, bit: bool) {
        let divisor = self.counts[i] as u32 + 2;
        let p = &mut self.probs[i];
        if bit {
            *p += (STATE_SCALE - *p) / divisor;
        } else {
            *p -= *p / divisor;
        }
        if self.counts[i] < COUNT_LIMIT {
            self.counts[i] += 1;
        }
    }
}
