use crate::mixer::Mixer;
use crate::model::ContextModel;

/// The context models that run side by side.
const ORDERS: [u32; 6] = [0, 1, 2, 3, 4, 6];

/// Asks every model for its prediction and lets the mixer combine them.
pub struct Predictor {
    models: Vec<ContextModel>,
    probs: Vec<u32>, // predictions of all models for the current bit
    mixer: Mixer,
}

impl Predictor {
    pub fn new() -> Self {
        Predictor {
            models: ORDERS
                .iter()
                .map(|&order| ContextModel::new(order))
                .collect(),
            probs: vec![0; ORDERS.len()],
            mixer: Mixer::new(ORDERS.len()),
        }
    }

    /// Probability that the next bit is 1.
    pub fn predict(&mut self) -> u32 {
        for (p, model) in self.probs.iter_mut().zip(&self.models) {
            *p = model.predict();
        }
        self.mixer.mix(&self.probs)
    }

    /// Everybody learns from the bit that really came.
    pub fn update(&mut self, bit: bool) {
        self.mixer.update(bit);
        for model in &mut self.models {
            model.update(bit);
        }
    }
}
