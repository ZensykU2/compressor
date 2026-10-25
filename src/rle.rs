pub fn encode(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let value = input[i];
        // Count how many equal bytes follow
        let mut run = 1;

        while i + run < input.len() && input[i + run] == value && run < 256 {
            run += 1;
        }

        output.push((run - 1) as u8);
        output.push(value);
        i += run;
    }
    output
}

pub fn decode(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    for pair in input.chunks_exact(2) {
        let run = pair[0] as usize + 1;
        let value = pair[1];
        for _ in 0..run {
            output.push(value);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: encode, decode, and check for original return
    fn roundtrip(data: &[u8]) {
        assert_eq!(decode(&encode(data)), data)
    }

    #[test]
    fn empty_input() {
        assert_eq!(encode(&[]), Vec::<u8>::new());
        roundtrip(&[]);
    }

    #[test]
    fn single_byte() {
        assert_eq!(encode(&[42]), vec![0, 42]);
        roundtrip(&[42]);
    }

    #[test]
    fn simple_runs() {
        assert_eq!(encode(&[7, 7, 7, 1, 1]), vec![2, 7, 1, 1]);
        roundtrip(&[7, 7, 7, 1, 1]);
    }

    #[test]
    fn long_run_is_split() {
        let data = vec![9u8; 300]; // 300 times the byte 9
        assert_eq!(encode(&data), vec![255, 9, 43, 9]); // 256 + 44 = 300
        roundtrip(&data);
    }

    #[test]
    fn all_byte_values() {
        let data: Vec<u8> = (0..=255).collect(); // collect() again!
        roundtrip(&data);
    }
}
