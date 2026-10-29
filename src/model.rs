use crate::counter::CounterTable;

pub const NODES: usize = 1 << u8::BITS;

/// The history is a u64, so it holds at most 7 bytes.
pub const MAX_ORDER: u32 = 7;

/// The table has 2^TABLE_BITS slots. Each context gets a block of NODES slots,
/// so there are 2^BLOCK_BITS blocks.
const TABLE_BITS: u32 = 24;
const BLOCK_BITS: u32 = TABLE_BITS - u8::BITS;

/// Golden ratio * 2^64. Multiplying by it mixes all input bits into the top bits.
pub const HASH_MULTIPLIER: u64 = 0x9E37_79B9_7F4A_7C15;

pub struct ContextModel {
    counters: CounterTable,
    history: u64,      // the last `order` bytes
    history_mask: u64, // keeps only those bytes
    block_bits: u32,   // how many bits choose the block
    hashed: bool,      // false: history is the block number directly
    block: usize,      // block of the current context
    node: usize,       // position in the bit tree (1..=255)
}

impl ContextModel {
    pub fn new(order: u32) -> Self {
        assert!(order <= MAX_ORDER, "order too large");
        let context_bits = u8::BITS * order;
        let block_bits = context_bits.min(BLOCK_BITS);
        let size = 1 << (block_bits + u8::BITS);
        ContextModel {
            counters: CounterTable::new(size),
            history: 0,
            history_mask: (1 << context_bits) - 1,
            block_bits,
            hashed: context_bits > BLOCK_BITS,
            block: 0,
            node: 1,
        }
    }

    /// Which block belongs to the current history.
    fn block_of(&self) -> usize {
        if self.hashed {
            // Take the TOP bits of the product: they depend on every input bit
            (self.history.wrapping_mul(HASH_MULTIPLIER) >> (u64::BITS - self.block_bits)) as usize
        } else {
            self.history as usize
        }
    }

    /// Where in the table the current context lives: [block][node]
    fn index(&self) -> usize {
        (self.block << u8::BITS) | self.node
    }

    /// Probability that the next bit is 1, in the coder's units.
    pub fn predict(&self) -> u32 {
        self.counters.predict(self.index())
    }

    /// Learn from the bit that really came, then move to the next context.
    pub fn update(&mut self, bit: bool) {
        let i = self.index();
        self.counters.update(i, bit);

        self.node = self.node * 2 + bit as usize;
        if self.node >= NODES {
            let byte = (self.node - NODES) as u64;
            self.history = ((self.history << u8::BITS) | byte) & self.history_mask;
            self.block = self.block_of(); // hash once per byte, not per bit
            self.node = 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*
    use crate::arithmetic::{PROB_MAX, PROB_MIN, PROB_SCALE};

    /// Feeds one byte into the model and returns how many bits it would cost.
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
        for order in 0..=MAX_ORDER {
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
    fn first_observation_counts_a_lot() {
        let mut model = ContextModel::new(0);
        cost_in_bits(&mut model, 0xFF); // the first bit of a byte was a 1 once
        // One 1 seen: 50 % -> 75 %, not just 50 % -> 53 % as with a fixed rate
        assert_eq!(model.predict(), PROB_SCALE * 3 / 4);
    }

    #[test]
    fn order3_sees_further_back() {
        let mut order2 = ContextModel::new(2);
        let mut order3 = ContextModel::new(3);
        // After "ab" comes '1' or '2'. Only the byte BEFORE "ab" tells which one.
        for _ in 0..1000 {
            for &byte in b"xab1yab2" {
                cost_in_bits(&mut order2, byte);
                cost_in_bits(&mut order3, byte);
            }
        }
        for &byte in b"xab" {
            cost_in_bits(&mut order2, byte);
            cost_in_bits(&mut order3, byte);
        }
        let cost2 = cost_in_bits(&mut order2, b'1');
        let cost3 = cost_in_bits(&mut order3, b'1');
        println!("'1' after \"xab\": order-2 {cost2:.3} bits, order-3 {cost3:.3} bits");
        assert!(cost2 > 0.5);
        assert!(cost3 < 0.1);
    }

    #[test]
    fn prediction_stays_in_valid_range() {
        for order in 0..=MAX_ORDER {
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
