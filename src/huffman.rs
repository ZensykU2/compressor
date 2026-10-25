use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use crate::bits::{BitReader, BitWriter};

/// Counts frequency of reoccurring byte values.
pub fn count_frequencies(data: &[u8]) -> [u64; 256] {
    let mut freqs = [0u64; 256];
    for &byte in data {
        freqs[byte as usize] += 1;
    }
    freqs
}

/// Computes the Huffman code length in bits for every byte value.
/// Bytes that never appear get length 0.
pub fn code_lengths(freqs: &[u64; 256]) -> [u8; 256] {
    let mut lengths = [0u8; 256];

    // Every heap entry is a group of symbols together with their total frequency.
    // BinaryHeap always gives the LARGEST entry first, Reverse flips that,
    // so pop() gives us the SMALLEST group.
    let mut heap = BinaryHeap::new();
    for symbol in 0..256 {
        if freqs[symbol] > 0 {
            heap.push(Reverse((freqs[symbol], vec![symbol as u8])));
        }
    }

    // Special case: only one distinct byte. It still needs 1 bit.
    if heap.len() == 1 {
        let Reverse((_, symbols)) = heap.pop().unwrap();
        lengths[symbols[0] as usize] = 1;
        return lengths;
    }

    // Merge the two smallest groups until only one is left.
    // Every merge adds one bit to every symbol inside the two groups.
    while heap.len() > 1 {
        let Reverse((freq_a, mut symbols_a)) = heap.pop().unwrap();
        let Reverse((freq_b, symbols_b)) = heap.pop().unwrap();
        for &symbol in symbols_a.iter().chain(symbols_b.iter()) {
            lengths[symbol as usize] += 1;
        }
        symbols_a.extend(symbols_b);
        heap.push(Reverse((freq_a + freq_b, symbols_a)));
    }
    lengths
}

pub fn canonical_codes(lengths: &[u8; 256]) -> [u32; 256] {
    let max_length = *lengths.iter().max().unwrap();
    assert!(max_length <= 32, "Huffman code longer than 32 bits");

    let mut codes = [0u32; 256];
    let mut code: u64 = 0;
    for length in 1..=max_length {
        for symbol in 0..256 {
            if lengths[symbol] == length {
                codes[symbol] = code as u32;
                code += 1;
            }
        }
        code <<= 1;
    }
    codes
}

const HEADER_SIZE: usize = 8 + 256;

pub fn encode(data: &[u8]) -> Vec<u8> {
    let freqs = count_frequencies(data);
    let lengths = code_lengths(&freqs);
    let codes = canonical_codes(&lengths);

    let mut output = Vec::new();
    output.extend_from_slice(&(data.len() as u64).to_le_bytes());
    output.extend_from_slice(&lengths);

    let mut writer = BitWriter::new();
    for &byte in data {
        writer.write_bits(codes[byte as usize], lengths[byte as usize]);
    }
    output.extend(writer.finish());
    output
}

pub fn decode(input: &[u8]) -> Option<Vec<u8>> {
    if input.len() < HEADER_SIZE {
        return None;
    }

    let original_len = u64::from_le_bytes(input[0..8].try_into().unwrap()) as usize;
    let lengths: [u8; 256] = input[8..HEADER_SIZE].try_into().unwrap();
    let body = &input[HEADER_SIZE..];

    if original_len > body.len() * 8 {
        return None;
    }

    let codes = canonical_codes(&lengths);
    let mut table = HashMap::new();
    for symbol in 0..256 {
        if lengths[symbol] > 0 {
            table.insert((lengths[symbol], codes[symbol]), symbol as u8);
        }
    }

    let mut reader = BitReader::new(body);
    let mut output = Vec::with_capacity(original_len);
    while output.len() < original_len {
        let mut code = 0u32;
        let mut length = 0u8;
        loop {
            let bit = reader.read_bit()?;
            code = (code << 1) | bit as u32;
            length += 1;
            if let Some(&symbol) = table.get(&(length, code)) {
                output.push(symbol);
                break;
            }
            if length >= 32 {
                return None;
            }
        }
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_example() {
        let freqs = count_frequencies(b"aaaabbc");
        let lengths = code_lengths(&freqs);
        assert_eq!(lengths[b'a' as usize], 1);
        assert_eq!(lengths[b'b' as usize], 2);
        assert_eq!(lengths[b'c' as usize], 2);
        assert_eq!(lengths[b'z' as usize], 0); // never appears

        let codes = canonical_codes(&lengths);
        assert_eq!(codes[b'a' as usize], 0b0);
        assert_eq!(codes[b'b' as usize], 0b10);
        assert_eq!(codes[b'c' as usize], 0b11);
    }

    #[test]
    fn single_symbol_gets_one_bit() {
        let lengths = code_lengths(&count_frequencies(b"zzzz"));
        assert_eq!(lengths[b'z' as usize], 1);
    }

    #[test]
    fn empty_input_has_no_codes() {
        let lengths = code_lengths(&count_frequencies(b""));
        assert!(lengths.iter().all(|&l| l == 0));
        canonical_codes(&lengths); // must not crash
    }

    #[test]
    fn kraft_sum_is_exactly_one() {
        let text = b"the quick brown fox jumps over the lazy dog, again and again";
        let lengths = code_lengths(&count_frequencies(text));
        let sum: u64 = lengths
            .iter()
            .filter(|&&l| l > 0)
            .map(|&l| 1u64 << (32 - l))
            .sum();
        assert_eq!(sum, 1u64 << 32);
    }

    fn roundtrip(data: &[u8]) {
        assert_eq!(decode(&encode(data)), Some(data.to_vec()));
    }

    #[test]
    fn roundtrip_special_cases() {
        roundtrip(b"");
        roundtrip(b"x");
        roundtrip(b"zzzz");
        roundtrip(b"aaaabbc");
        roundtrip(b"the quick brown fox jumps over the lazy dog");
        let all_bytes: Vec<u8> = (0..=255).collect();
        roundtrip(&all_bytes);
    }

    #[test]
    fn padding_does_not_add_symbols() {
        let compressed = encode(b"aab");
        assert_eq!(compressed.len(), HEADER_SIZE + 1);
        assert_eq!(decode(&compressed), Some(b"aab".to_vec()));
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
        roundtrip(&data);
    }

    #[test]
    fn too_short_input_is_rejected() {
        assert_eq!(decode(&[1, 2, 3]), None);
    }
}
