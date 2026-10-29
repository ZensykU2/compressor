use crate::match_model::MatchModel;
use crate::mixer::Mixer;
use crate::model::{ContextModel, NODES};

/// The context models that run side by side.
const ORDERS: [u32; 6] = [0, 1, 2, 3, 4, 6];

/// One weight set per position in the bit tree.
const WEIGHT_SETS: usize = NODES;

/// Asks every model for its prediction and lets the mixer combine them.
pub struct Predictor {
    models: Vec<ContextModel>,
    probs: Vec<u32>, // predictions of all models for the current bit
    match_model: MatchModel,
    mixer: Mixer,
    node: usize, // position in the bit tree of the current byte
}

impl Predictor {
    pub fn new() -> Self {
        Predictor {
            models: ORDERS
                .iter()
                .map(|&order| ContextModel::new(order))
                .collect(),
            probs: vec![0; ORDERS.len() + 1],
            match_model: MatchModel::new(),
            mixer: Mixer::new(ORDERS.len() + 1, WEIGHT_SETS),
            node: 1,
        }
    }

    /// Probability that the next bit is 1.
    pub fn predict(&mut self) -> u32 {
        for (p, model) in self.probs.iter_mut().zip(&self.models) {
            *p = model.predict();
        }
        self.probs[ORDERS.len()] = self.match_model.predict();
        self.mixer.mix(&self.probs, self.node)
    }

    /// Everybody learns from the bit that really came.
    pub fn update(&mut self, bit: bool) {
        self.mixer.update(bit);
        for model in &mut self.models {
            model.update(bit);
        }
        self.match_model.update(bit);
        self.node = self.node * 2 + bit as usize;
        if self.node >= NODES {
            self.node = 1;
        }
    }
}
