use crate::arithmetic::PROB_SCALE;
use crate::counter::CounterTable;
use crate::model::{HASH_MULTIPLIER, NODES};

/// A match must be at least this long before trusting it.
const MIN_MATCH: usize = 6;

/// The hash table remembers 2^MATCH_TABLE_BITS positions.
const MATCH_TABLE_BITS: u32 = 22;

/// Longer matches all share the last length bucket.
const MAX_LENGTH_BUCKET: usize = 15;

/// Finds the last time the recent bytes appeared and predicts that
/// the same byte follows again
pub struct MatchModel {
    history: Vec<u8>,
    table: Vec<u32>,
    match_ptr: usize,
    length: usize,
    node: usize,
    bit_count: u32,
    counters: CounterTable,
}

impl MatchModel {
    pub fn new() -> Self {
        MatchModel {
            history: Vec::new(),
            table: vec![0; 1 << MATCH_TABLE_BITS],
            match_ptr: 0,
            length: 0,
            node: 1,
            bit_count: 0,
            counters: CounterTable::new((MAX_LENGTH_BUCKET + 1) * 2),
        }
    }

    /// The bit the match predicts for the current position.
    fn expected_bit(&self) -> usize {
        let byte = self.history[self.match_ptr];
        ((byte >> (u8::BITS - 1 - self.bit_count)) & 1) as usize
    }

    fn slot(&self) -> usize {
        self.length.min(MAX_LENGTH_BUCKET) * 2 + self.expected_bit()
    }

    pub fn predict(&self) -> u32 {
        if self.length == 0 {
            return PROB_SCALE / 2;
        }
        self.counters.predict(self.slot())
    }

    pub fn update(&mut self, bit: bool) {
        if self.length > 0 {
            let slot = self.slot();
            let expected = self.expected_bit() == 1;
            self.counters.update(slot, bit);
            if bit != expected {
                self.length = 0;
            }
        }

        self.node = self.node * 2 + bit as usize;
        self.bit_count += 1;
        if self.bit_count == u8::BITS {
            let byte = (self.node - NODES) as u8;
            self.history.push(byte);
            self.node = 1;
            self.bit_count = 0;
            if self.length > 0 {
                self.length += 1;
                self.match_ptr += 1;
            }
            self.find_match();
        }
    }

    /// Hash of the last MIN_MATCH bytes.
    fn hash(&self) -> usize {
        let end = self.history.len();
        let mut h: u64 = 0;
        for &byte in &self.history[end - MIN_MATCH..] {
            h = (h << u8::BITS) | byte as u64;
        }
        (h.wrapping_mul(HASH_MULTIPLIER) >> (u64::BITS - MATCH_TABLE_BITS)) as usize
    }

    fn find_match(&mut self) {
        let end = self.history.len();
        if end < MIN_MATCH {
            return;
        }
        let h = self.hash();
        if self.length == 0 {
            let candidate = self.table[h] as usize;
            if candidate > 0 {
                // Check how many bytes really match, hashes could collide
                let mut length = 0;
                while length < candidate
                    && self.history[candidate - 1 - length] == self.history[end - 1 - length]
                {
                    length += 1;
                }
                if length >= MIN_MATCH {
                    self.length = length;
                    self.match_ptr = candidate;
                }
            }
        }
        self.table[h] = end as u32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feeds one byte and returns how many bits it would cost (test only, uses f64).
    fn cost_in_bits(model: &mut MatchModel, byte: u8) -> f64 {
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
    fn no_opinion_at_the_start() {
        assert_eq!(MatchModel::new().predict(), PROB_SCALE / 2);
    }

    #[test]
    fn repeated_text_gets_cheap() {
        let mut model = MatchModel::new();
        let text = b"Context mixing predicts every single bit. The match model finds long repeats.";
        let first: f64 = text
            .iter()
            .map(|&byte| cost_in_bits(&mut model, byte))
            .sum();
        let second: f64 = text
            .iter()
            .map(|&byte| cost_in_bits(&mut model, byte))
            .sum();
        println!("first time {first:.0} bits, second time {second:.0} bits");
        assert!(second < first / 4.0);
    }

    #[test]
    fn a_wrong_byte_ends_the_match() {
        let mut model = MatchModel::new();
        // Repeat a few times, so the counters learn that matches are reliable
        for _ in 0..5 {
            for &byte in b"abcdefghij" {
                cost_in_bits(&mut model, byte);
            }
        }
        for &byte in b"abcdefgh" {
            cost_in_bits(&mut model, byte);
        }
        assert_ne!(model.predict(), PROB_SCALE / 2); // has an opinion: 'i' comes
        cost_in_bits(&mut model, b'Z'); // but 'Z' comes
        assert_eq!(model.predict(), PROB_SCALE / 2); // no match anymore
    }
}
