/// Probabilities are 12-bit numbers: 0 means 0 %, PROB_SCALE means 100 %.
pub const PROB_BITS: u32 = 12;
pub const PROB_SCALE: u32 = 1 << PROB_BITS; // 4096

/// A model must never say "impossible", so p1 must stay in this range.
pub const PROB_MIN: u32 = 1;
pub const PROB_MAX: u32 = PROB_SCALE - 1; // 4095

const TOP_BYTE_MASK: u32 = 0xFF00_0000;

/// True if `low` and `high` start with the same byte.
/// Every number between them starts with it too, so it can never change again.
fn top_byte_is_settled(low: u32, high: u32) -> bool {
    (low ^ high) & TOP_BYTE_MASK == 0
}

/// Binary arithmetic encoder.
/// Current interval is [low, high]. Every bit makes it smaller.
pub struct Encoder {
    low: u32,
    high: u32,
    output: Vec<u8>,
}

impl Encoder {
    pub fn new() -> Self {
        Encoder {
            low: 0,
            high: u32::MAX,
            output: Vec::new(),
        }
    }

    /// Encodes one bit. `p1` is the probability that the bit is 1,
    /// in units of 1/PROB_SCALE (must be PROB_MIN..=PROB_MAX).
    pub fn encode(&mut self, bit: bool, p1: u32) {
        debug_assert!(
            (PROB_MIN..=PROB_MAX).contains(&p1),
            "p1 must be in PROB_MIN..=PROB_MAX"
        );

        // Split the interval: the lower part [low, mid] belongs to bit 1,
        // the upper part [mid + 1, high] belongs to bit 0.
        let range = self.high - self.low;
        let mid = self.low + (range >> PROB_BITS) * p1;
        if bit {
            self.high = mid;
        } else {
            self.low = mid + 1;
        }

        // Write out every top byte that can never change again.
        while top_byte_is_settled(self.low, self.high) {
            self.output.push((self.high >> 24) as u8); // the top byte
            self.low <<= 8;
            self.high = (self.high << 8) | 0xFF;
        }
    }

    /// Writes the remaining bytes. `low` lies inside the final interval,
    /// so the decoder will land in the right place.
    pub fn finish(mut self) -> Vec<u8> {
        self.output.extend_from_slice(&self.low.to_be_bytes());
        self.output
    }
}

/// Binary arithmetic decoder. Makes exactly the same splits as the encoder
/// and checks on which side `x` lies.
pub struct Decoder<'a> {
    low: u32,
    high: u32,
    x: u32, // window into the compressed data, as wide as low and high
    input: &'a [u8],
    pos: usize,
}

impl<'a> Decoder<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        let mut decoder = Decoder {
            low: 0,
            high: u32::MAX,
            x: 0,
            input,
            pos: 0,
        };
        // Fill the window `x` with the first bytes (a u32 holds 4 of them)
        for _ in 0..size_of::<u32>() {
            decoder.x = (decoder.x << 8) | decoder.next_byte() as u32;
        }
        decoder
    }

    pub fn is_past_end(&self) -> bool {
        self.pos > self.input.len()
    }

    /// Next input byte, or 0 if we are past the end.
    fn next_byte(&mut self) -> u8 {
        let byte = self.input.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        byte
    }

    /// Decodes one bit. `p1` must be exactly the same value the encoder used.
    pub fn decode(&mut self, p1: u32) -> bool {
        let range = self.high - self.low;
        let mid = self.low + (range >> PROB_BITS) * p1;
        let bit = self.x <= mid;
        if bit {
            self.high = mid;
        } else {
            self.low = mid + 1;
        }

        while top_byte_is_settled(self.low, self.high) {
            self.low <<= 8;
            self.high = (self.high << 8) | 0xFF;
            self.x = (self.x << 8) | self.next_byte() as u32;
        }
        bit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates `count` pseudo-random bits, about `percent_ones` % of them are 1.
    fn random_bits(count: usize, percent_ones: u32, seed: u32) -> Vec<bool> {
        let mut x = seed;
        (0..count)
            .map(|_| {
                x = x.wrapping_mul(1103515245).wrapping_add(12345);
                (x >> 16) % 100 < percent_ones
            })
            .collect()
    }

    /// Encodes all bits with a fixed probability, decodes them again,
    /// checks every bit and returns the compressed size.
    fn roundtrip(bits: &[bool], p1: u32) -> usize {
        let mut encoder = Encoder::new();
        for &bit in bits {
            encoder.encode(bit, p1);
        }
        let compressed = encoder.finish();

        let mut decoder = Decoder::new(&compressed);
        for (i, &bit) in bits.iter().enumerate() {
            assert_eq!(decoder.decode(p1), bit, "wrong bit at position {i}");
            assert!(!decoder.is_past_end(), "decoder read past the end");
        }
        compressed.len()
    }

    #[test]
    fn empty_and_tiny_inputs() {
        roundtrip(&[], PROB_SCALE / 2);
        roundtrip(&[true], PROB_SCALE / 2);
        roundtrip(&[false], PROB_SCALE / 2);
        roundtrip(&[true, false, true], PROB_SCALE / 4);
    }

    #[test]
    fn many_probabilities() {
        // Every combination of "what we predict" and "what really comes",
        // including the extreme values at both ends.
        let predictions = [
            PROB_MIN,
            PROB_MIN + 1,
            PROB_SCALE / 40,      // 2.5 %
            PROB_SCALE / 4,       // 25 %
            PROB_SCALE / 2,       // 50 %
            PROB_SCALE * 3 / 4,   // 75 %
            PROB_SCALE * 39 / 40, // 97.5 %
            PROB_MAX - 1,
            PROB_MAX,
        ];
        for p1 in predictions {
            for percent in [0, 10, 50, 90, 100] {
                roundtrip(&random_bits(5000, percent, p1), p1);
            }
        }
    }

    #[test]
    fn good_prediction_compresses() {
        // 80 000 bits, 10 % ones, predicted as 10 %
        let bits = random_bits(80_000, 10, 42);
        let size = roundtrip(&bits, PROB_SCALE / 10);
        println!("80000 bits with 10% ones -> {size} bytes");
        assert!(size < 5000, "got {size} bytes");
    }

    #[test]
    fn wrong_prediction_still_roundtrips() {
        // 1000 ones, but we claim a 1 is almost impossible
        let bits = vec![true; 1000];
        let size = roundtrip(&bits, PROB_MIN);
        println!("1000 ones predicted as almost impossible -> {size} bytes");
    }
}
