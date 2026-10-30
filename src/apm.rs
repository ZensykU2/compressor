use crate::arithmetic::{PROB_BITS, PROB_MAX, PROB_MIN};
use crate::mixer::{STRETCH_MAX, squash, stretch};

/// Support points per context: one every 0.5 in the stretched domain.
const POINTS: usize = 33;

/// Distance between two support points: 2^7 = 128 = 0.5.
const STEP_BITS: u32 = 7;
const STEP: u32 = 1 << STEP_BITS;

/// Inside the table, probabilities have 16 bits.
const APM_BITS: u32 = 16;

/// How fast the table learns: BIGGER = SLOWER.
const APM_RATE: u32 = 6;

/// Adaptive Probability Map.
/// Takes a finished prediction and corrects it, seperately for every context.
pub struct Apm {
    table: Vec<u32>,
    index: usize,
}

impl Apm {
    pub fn new(contexts: usize) -> Self {
        let mut table = Vec::with_capacity(contexts * POINTS);
        for _ in 0..contexts {
            for j in 0..POINTS {
                // Start as no correction: output = input
                let x = (j as i32 - (POINTS as i32 / 2)) * STEP as i32;
                table.push(squash(x) << (APM_BITS - PROB_BITS));
            }
        }
        Apm { table, index: 0 }
    }

    /// Corrects probability `p` in context `context`.
    pub fn refine(&mut self, p: u32, context: usize) -> u32 {
        let x = (stretch(p) + STRETCH_MAX + 1) as u32; // 1..=4095
        let base = context * POINTS + (x >> STEP_BITS) as usize;
        let w = x & (STEP - 1); // how far between point base and base + 1
        // The nearer point learns from the next bit
        self.index = if w < STEP / 2 { base } else { base + 1 };
        let p = (self.table[base] * (STEP - w) + self.table[base + 1] * w)
            >> (STEP_BITS + APM_BITS - PROB_BITS);
        p.clamp(PROB_MIN, PROB_MAX)
    }

    pub fn update(&mut self, bit: bool) {
        let t = &mut self.table[self.index];
        if bit {
            *t += ((1 << APM_BITS) - *t) >> APM_RATE;
        } else {
            *t -= *t >> APM_RATE;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arithmetic::PROB_SCALE;

    #[test]
    fn starts_without_correction() {
        let mut apm = Apm::new(1);
        for p in [PROB_MIN, 100, 1000, 2048, 3000, 4000, PROB_MAX] {
            let refined = apm.refine(p, 0);
            assert!(refined.abs_diff(p) <= p / 50 + 1, "{p} became {refined}");
        }
    }

    #[test]
    fn learns_a_correction() {
        // Context 0: the input always says 50 %, but 9 of 10 bits are 1.
        let mut apm = Apm::new(2);
        let half = PROB_SCALE / 2;
        for i in 0..10_000 {
            apm.refine(half, 0);
            apm.update(i % 10 != 0);
        }
        let in_context0 = apm.refine(half, 0);
        let in_context1 = apm.refine(half, 1);
        println!("50 % becomes {in_context0} in context 0, {in_context1} in context 1");
        assert!(in_context0 > PROB_SCALE * 85 / 100); // learned: really about 90 %
        assert_eq!(in_context1, half); // other contexts are untouched
    }
}
