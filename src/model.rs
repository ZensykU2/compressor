use crate::arithmetic::{PROB_BITS, PROB_MAX, PROB_MIN};

const STATE_BITS: u32 = 16;
const STATE_SCALE: u32 = 1 << STATE_BITS;

const _: () = assert!(
    STATE_BITS >= PROB_BITS,
    "STATE_BITS must be atleast PROB_BITS"
);

const LEARNING_RATE: u32 = 4;

const NODES: usize = 1 << u8::BITS;

/// Up to this order in the table fits into memory directly.
/// Higher orders need a hash table.
pub const MAX_DIRECT_ORDER: u32 = 2;

pub struct ContextModel {
    probs: Vec<u32>,
    order: u32,
    history: usize,
    node: usize,
}

impl ContextModel {
    pub fn new(order: u32) -> Self {
        assert!(
            order <= MAX_DIRECT_ORDER,
            "order too large for a direct table"
        );

        let size = 1 << (u8::BITS * (order + 1));
        ContextModel {
            probs: vec![STATE_SCALE / 2; size],
            order,
            history: 0,
            node: 1,
        }
    }

    /// Where in the table the current context lives: [history bytes][node]
    fn index(&self) -> usize {
        (self.history << u8::BITS) | self.node
    }

    /// Probability that the next bit is 1, in the coder's units.
    pub fn predict(&self) -> u32 {
        let p = self.probs[self.index()] >> (STATE_BITS - PROB_BITS);
        p.clamp(PROB_MIN, PROB_MAX)
    }

    /// Learn from the bit that really came, then move to the next context.
    pub fn update(&mut self, bit: bool) {
        let i = self.index();
        let p = &mut self.probs[i];
        if bit {
            *p += (STATE_SCALE - *p) >> LEARNING_RATE;
        } else {
            *p -= *p >> LEARNING_RATE;
        }

        self.node = self.node * 2 + bit as usize;
        if self.node >= NODES {
            let byte = self.node - NODES;
            // Push it into the history and keep only the last order bytes
            let mask = (1 << (u8::BITS * self.order)) - 1;
            self.history = ((self.history << u8::BITS) | byte) & mask;
            self.node = 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arithmetic::PROB_SCALE;

    /// Feeds one byte into the model and returns how many bits it would cost.
    /// (Only for tests: uses f64, which the real coder must never do.)
    fn cost_in_bits(model: &mut ContextModel, byte: u8) -> f64 {
        let mut bits = 0.0;
        for i in (0..u8::BITS).rev() {
            let bit = (byte >> i) & 1 == 1;
            let p1 = model.predict() as f64 / PROB_SCALE as f64;
            let p = if bit { p1 } else { 1.0 - p1 };
            bits -= p.log2();
            model.update(bit);
        }
        bits
    }

    #[test]
    fn starts_at_fifty_percent() {
        for order in 0..=MAX_DIRECT_ORDER {
            assert_eq!(ContextModel::new(order).predict(), PROB_SCALE / 2);
        }
    }

    #[test]
    fn order1_learns_what_follows_what() {
        let mut order0 = ContextModel::new(0);
        let mut order1 = ContextModel::new(1);
        // Train both on "abababab..."
        for _ in 0..1000 {
            for &byte in b"ab" {
                cost_in_bits(&mut order0, byte);
                cost_in_bits(&mut order1, byte);
            }
        }
        // How much does a 'b' cost right after an 'a'?
        cost_in_bits(&mut order0, b'a');
        cost_in_bits(&mut order1, b'a');
        let cost0 = cost_in_bits(&mut order0, b'b');
        let cost1 = cost_in_bits(&mut order1, b'b');
        println!("'b' after 'a': order-0 {cost0:.3} bits, order-1 {cost1:.3} bits");
        assert!(cost0 > 0.5);
        assert!(cost1 < 0.1);
    }

    #[test]
    fn prediction_stays_in_valid_range() {
        for order in 0..=MAX_DIRECT_ORDER {
            let mut model = ContextModel::new(order);
            for i in 0..10_000 {
                model.update(i % 3 == 0);
                assert!((PROB_MIN..=PROB_MAX).contains(&model.predict()));
            }
            let mut model = ContextModel::new(order);
            for _ in 0..10_000 {
                model.update(false);
                assert!((PROB_MIN..=PROB_MAX).contains(&model.predict()));
            }
        }
    }
}
