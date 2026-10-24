pub struct BitWriter {
    bytes: Vec<u8>,
    current: u8,
    count: u8,
}

impl BitWriter {
    pub fn new() -> Self {
        BitWriter {
            bytes: Vec::new(),
            current: 0,
            count: 0,
        }
    }

    pub fn write_bit(&mut self, bit: bool) {
        self.current = (self.current << 1) | (bit as u8);
        self.count += 1;
        if self.count == 8 {
            self.bytes.push(self.current);
            self.current = 0;
            self.count = 0;
        }
    }

    pub fn write_bits(&mut self, value: u32, n: u8) {
        assert!(n <= 32, "write_bits: at most 32 bits");
        for i in (0..n).rev() {
            let bit = (value >> i) & 1 == 1;
            self.write_bit(bit);
        }
    }

    pub fn finish(mut self) -> Vec<u8> {
        if self.count > 0 {
            self.current <<= 8 - self.count;
            self.bytes.push(self.current);
        }
        self.bytes
    }
}

pub struct BitReader<'a> {
    bytes: &'a [u8],
    pos: usize,
    bit: u8,
}

impl<'a> BitReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        BitReader {
            bytes,
            pos: 0,
            bit: 0,
        }
    }

    pub fn read_bit(&mut self) -> Option<bool> {
        let byte = *self.bytes.get(self.pos)?;
        let bit = (byte >> (7 - self.bit)) & 1 == 1;
        self.bit += 1;
        if self.bit == 8 {
            self.bit = 0;
            self.pos += 1;
        }
        Some(bit)
    }

    pub fn read_bits(&mut self, n: u8) -> Option<u32> {
        assert!(n <= 32, "read_bits: at most 32 bits");
        let mut value = 0u32;
        for _ in 0..n {
            let bit = self.read_bit()?;
            value = (value << 1) | (bit as u32);
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_bits_roundtrip() {
        let pattern = [true, false, true, true, false, false, true, false, true];
        let mut writer = BitWriter::new();
        for &bit in &pattern {
            writer.write_bit(bit);
        }
        let bytes = writer.finish();

        assert_eq!(bytes.len(), 2);
        assert_eq!(bytes[0], 0b1011_0010);
        assert_eq!(bytes[1], 0b1000_0000);

        let mut reader = BitReader::new(&bytes);
        for &bit in &pattern {
            assert_eq!(reader.read_bit(), Some(bit));
        }
    }

    #[test]
    fn multi_bit_values_roundtrip() {
        let mut writer = BitWriter::new();
        writer.write_bits(5, 3);
        writer.write_bits(0, 1);
        writer.write_bits(1000, 10);
        writer.write_bits(0xFFFF_FFFF, 32);
        let bytes = writer.finish();

        let mut reader = BitReader::new(&bytes);
        assert_eq!(reader.read_bits(3), Some(5));
        assert_eq!(reader.read_bits(1), Some(0));
        assert_eq!(reader.read_bits(10), Some(1000));
        assert_eq!(reader.read_bits(32), Some(0xFFFF_FFFF));
    }

    #[test]
    fn empty_input_returns_none() {
        let mut reader = BitReader::new(&[]);
        assert_eq!(reader.read_bit(), None);
    }
}
