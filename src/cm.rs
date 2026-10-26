use crate::arithmetic::{Decoder, Encoder};
use crate::model::Order0;

const LENGTH_SIZE: usize = size_of::<u64>();

pub fn encode(data: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    output.extend_from_slice(&(data.len() as u64).to_le_bytes());

    let mut model = Order0::new();
    let mut encoder = Encoder::new();
    for &byte in data {
        for i in (0..u8::BITS).rev() {
            let bit = (byte >> i) & 1 == 1;
            encoder.encode(bit, model.predict());
            model.update(bit);
        }
    }
    output.extend(encoder.finish());
    output
}

pub fn decode(input: &[u8]) -> Option<Vec<u8>> {
    if input.len() < LENGTH_SIZE {
        return None;
    }
    let original_len = u64::from_le_bytes(input[..LENGTH_SIZE].try_into().unwrap()) as usize;

    let mut model = Order0::new();
    let mut decoder = Decoder::new(&input[LENGTH_SIZE..]);
    let mut output = Vec::new();
    for _ in 0..original_len {
        let mut byte = 0u8;
        for _ in 0..u8::BITS {
            let bit = decoder.decode(model.predict());
            model.update(bit);
            byte = (byte << 1) | bit as u8;
        }
        output.push(byte)
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(data: &[u8]) -> usize {
        let compressed = encode(data);
        assert_eq!(decode(&compressed), Some(data.to_vec()));
        compressed.len()
    }

    #[test]
    fn roundtrip_special_cases() {
        roundtrip(b"");
        roundtrip(b"x");
        roundtrip(b"zzzz");
        roundtrip(b"the quick brown fox jumps over the lazy dog");
        let all_bytes: Vec<u8> = (0..=255).collect();
        roundtrip(&all_bytes);
    }

    #[test]
    fn pseudo_random_data() {
        let mut x: u32 = 12345;
        let data: Vec<u8> = (0..10_000)
            .map(|_| {
                x = x.wrapping_mul(1103515245).wrapping_add(12345);
                (x >> 16) as u8
            })
            .collect();
        let size = roundtrip(&data);
        println!("10000 random bytes -> {size} bytes");
    }

    #[test]
    fn predictable_data_gets_tiny() {
        let size = roundtrip(&vec![0u8; 10_000]);
        println!("10000 zero bytes -> {size} bytes");
        assert!(size < 200, "got {size} bytes");
    }

    #[test]
    fn too_short_input_is_rejected() {
        assert_eq!(decode(&[1, 2, 3]), None);
    }
}
