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
            high: 0xFFFF_FFFF,
            output: Vec::new(),
        }
    }

    /// Encodes one bit. `p1` is the probability that the bit is 1,
    /// in units of 1/4096.
    pub fn encode(&mut self, bit: bool, p1: u32) {
        debug_assert!(p1 > 0 && p1 < 4096, "p1 must be in 1..=4095");

        let range = self.high - self.low;
        let mid = self.low + (range >> 12) * p1;
        if bit {
            self.high = mid;
        } else {
            self.low = mid + 1;
        }

        // While the top byte of low and high is equal, it can never change again.
        while (self.low ^ self.high) & 0xFF00_0000 == 0 {
            self.output.push((self.high >> 24) as u8);
            self.low <<= 8;
            self.high = (self.high << 8) | 0xFF;
        }
    }

    /// Writes the remaining bytes.
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
    x: u32,
    input: &'a [u8],
    pos: usize,
}

impl<'a> Decoder<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        let mut decoder = Decoder {
            low: 0,
            high: 0xFFFF_FFFF,
            x: 0,
            input,
            pos: 0,
        };
        for _ in 0..4 {
            decoder.x = (decoder.x << 8) | decoder.next_byte() as u32;
        }
        decoder
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
        let mid = self.low + (range >> 12) * p1;
        let bit = self.x <= mid;
        if bit {
            self.high = mid;
        } else {
            self.low = mid + 1;
        }

        while (self.low ^ self.high) & 0xFF00_0000 == 0 {
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

    fn random_bits(count: usize, percent_ones: u32, seed: u32) -> Vec<bool> {
        let mut x = seed;
        (0..count)
            .map(|_| {
                x = x.wrapping_mul(1103515245).wrapping_add(12345);
                (x >> 16) % 100 < percent_ones
            })
            .collect()
    }

    fn roundtrip(bits: &[bool], p1: u32) -> usize {
        let mut encoder = Encoder::new();
        for &bit in bits {
            encoder.encode(bit, p1);
        }
        let compressed = encoder.finish();

        let mut decoder = Decoder::new(&compressed);
        for (i, &bit) in bits.iter().enumerate() {
            assert_eq!(decoder.decode(p1), bit, "wrong bit at position {i}");
        }
        compressed.len()
    }

    #[test]
    fn empty_and_tiny_inputs() {
        roundtrip(&[], 2048);
        roundtrip(&[true], 2048);
        roundtrip(&[false], 2048);
        roundtrip(&[true, false, true], 1000);
    }

    #[test]
    fn many_probabilities() {
        for p1 in [1, 2, 100, 1000, 2048, 3000, 4000, 4094, 4095] {
            for percent in [0, 10, 50, 90, 100] {
                roundtrip(&random_bits(5000, percent, p1), p1);
            }
        }
    }

    #[test]
    fn good_prediction_compresses() {
        let bits = random_bits(80_000, 10, 42);
        let size = roundtrip(&bits, 410);
        println!("80000 bits with 10% ones -> {size} bytes");
        assert!(size < 5000, "got {size} bytes");
    }

    #[test]
    fn wrong_prediction_still_roundtrips() {
        let bits = vec![true; 1000];
        let size = roundtrip(&bits, 1);
        println!("1000 ones predicted as almost impossible -> {size} bytes");
    }
}
