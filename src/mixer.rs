use crate::arithmetic::{PROB_BITS, PROB_MAX, PROB_MIN, PROB_SCALE};

// The squash table below is made for 12-bit probabilities.
const _: () = assert!(PROB_BITS == 12, "squash table expects PROB_BITS == 12");

pub const STRETCH_MAX: i32 = 2047;

const STEP_BITS: u32 = 7;
const STEP: i32 = 1 << STEP_BITS;

/// squash(x) = 4096 / (1 + e^-x) at x = -8.0, 7.5, ..., +8.0.
/// A test checks these numbers against the real formula.
const SQUASH_POINTS: [i32; 33] = [
    1, 2, 4, 6, 10, 17, 27, 45, 74, 120, 194, 311, 488, 747, 1102, 1546, 2048, 2550, 2994, 3349,
    3608, 3785, 3902, 3976, 4022, 4051, 4069, 4079, 4086, 4090, 4092, 4094, 4095,
];

/// Turns a logit back into a probability.
/// Between two support points interpolate linearly.
pub const fn squash(x: i32) -> u32 {
    let x = if x > STRETCH_MAX {
        STRETCH_MAX
    } else if x < -STRETCH_MAX {
        -STRETCH_MAX
    } else {
        x
    };
    let shifted = x + (STRETCH_MAX + 1);
    let i = (shifted >> STEP_BITS) as usize; // segment
    let w = shifted & (STEP - 1); // how far inside segment
    let p = (SQUASH_POINTS[i] * (STEP - w) + SQUASH_POINTS[i + 1] * w + STEP / 2) >> STEP_BITS;
    p as u32
}

/// stretch(p) = ln(p / (1 - p)), the inverse of squash. computed once at compile time.
const STRETCH_TABLE: [i32; PROB_SCALE as usize] = build_stretch_table();

const fn build_stretch_table() -> [i32; PROB_SCALE as usize] {
    let mut table = [0; PROB_SCALE as usize];
    let mut p = 0;
    let mut x = -STRETCH_MAX;
    // Walk through all logits from small to large. Every probability up to
    // squash(x) that has no entry yet gets x.
    while x <= STRETCH_MAX {
        let v = squash(x) as usize;
        while p <= v {
            table[p] = x;
            p += 1;
        }
        x += 1;
    }
    while p < PROB_SCALE as usize {
        table[p] = STRETCH_MAX;
        p += 1;
    }
    table
}

pub fn stretch(p: u32) -> i32 {
    STRETCH_TABLE[p as usize]
}

/// Weights are fixed point numbers: WEIGHT_ONE means 1.0.
const WEIGHT_BITS: u32 = 16;
const WEIGHT_ONE: i32 = 1 << WEIGHT_BITS;

/// How fast the weights learn: bigger shift = slower.
const MIXER_SHIFT: u32 = 12;

/// Combines the predictions of several models into one.
/// It learns while coding which model it can trust.
pub struct Mixer {
    weights: Vec<i32>, // one weight per model
    inputs: Vec<i32>,  // stretched predictions for the current bit
    output: u32,       // last mixed probability
}

impl Mixer {
    pub fn new(input_count: usize) -> Self {
        Mixer {
            // Start by trusting every model the same
            weights: vec![WEIGHT_ONE / input_count as i32; input_count],
            inputs: vec![0; input_count],
            output: PROB_SCALE / 2,
        }
    }

    /// p = squash(sum of weight_i * stretch(p_i))
    pub fn mix(&mut self, probs: &[u32]) -> u32 {
        debug_assert_eq!(probs.len(), self.inputs.len());
        let mut dot: i64 = 0;
        for i in 0..probs.len() {
            self.inputs[i] = stretch(probs[i]);
            dot += self.inputs[i] as i64 * self.weights[i] as i64;
        }
        let x = (dot >> WEIGHT_BITS).clamp(-STRETCH_MAX as i64, STRETCH_MAX as i64) as i32;
        self.output = squash(x).clamp(PROB_MIN, PROB_MAX);
        self.output
    }

    /// One step of gradient descent: weight_i += error * stretch(p_i).
    /// Models that pointed in the right direction get more weight.
    pub fn update(&mut self, bit: bool) {
        let error = ((bit as i32) << PROB_BITS) - self.output as i32;
        for (weight, &input) in self.weights.iter_mut().zip(&self.inputs) {
            *weight += (input * error) >> MIXER_SHIFT;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squash_points_match_formula() {
        for (i, &point) in SQUASH_POINTS.iter().enumerate() {
            let x = (i as f64 - 16.0) * 0.5;
            let exact = PROB_SCALE as f64 / (1.0 + (-x).exp());
            assert!(
                (point as f64 - exact).abs() <= 0.5,
                "point {i}: {point} vs {exact:.2}"
            );
        }
    }

    #[test]
    fn stretch_undoes_squash() {
        for x in -STRETCH_MAX..=STRETCH_MAX {
            let p = squash(x);
            // squash is flat at the ends, so many x give the same p.
            // stretch must at least land on an x with the same p.
            assert_eq!(squash(stretch(p)), p, "x = {x}");
        }
        assert_eq!(stretch(PROB_SCALE / 2), 0); // 50 % is exactly in the middle
    }

    #[test]
    fn mixer_learns_whom_to_trust() {
        // Model A always points the right way, but is only 90 % sure.
        // Model B is a liar: it always says the opposite.
        let mut mixer = Mixer::new(2);
        let says_one = PROB_SCALE * 9 / 10;
        let says_zero = PROB_SCALE / 10;
        for i in 0..10_000 {
            let bit = i % 3 == 0;
            let (a, b) = if bit {
                (says_one, says_zero)
            } else {
                (says_zero, says_one)
            };
            mixer.mix(&[a, b]);
            mixer.update(bit);
        }
        // Now A says 1 and B says 0. The mixer should be MORE sure than A alone:
        // it learned to believe A and to believe the opposite of B.
        let p = mixer.mix(&[says_one, says_zero]);
        println!(
            "A says 90 %, liar B says 10 % -> mixer says {:.1} %",
            p as f64 * 100.0 / PROB_SCALE as f64
        );
        assert!(p > says_one);
    }
}
