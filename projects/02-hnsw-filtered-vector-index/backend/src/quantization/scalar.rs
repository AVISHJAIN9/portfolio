//! Scalar Quantization (SQ8) for vector memory compression

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarQuantizer {
    pub min_val: f32,
    pub max_val: f32,
    pub scale: f32,
}

impl ScalarQuantizer {
    pub fn new(min_val: f32, max_val: f32) -> Self {
        let diff = (max_val - min_val).max(1e-6);
        let scale = 255.0 / diff;
        Self { min_val, max_val, scale }
    }

    pub fn quantize(&self, vector: &[f32]) -> Vec<u8> {
        vector.iter().map(|&v| {
            let clamped = v.clamp(self.min_val, self.max_val);
            ((clamped - self.min_val) * self.scale).round() as u8
        }).collect()
    }

    pub fn dequantize(&self, quantized: &[u8]) -> Vec<f32> {
        let inv_scale = 1.0 / self.scale;
        quantized.iter().map(|&b| {
            self.min_val + (b as f32) * inv_scale
        }).collect()
    }
}
