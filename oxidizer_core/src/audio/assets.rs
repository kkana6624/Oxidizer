use std::sync::Arc;
use anyhow::{Result, bail};

#[derive(Debug, Clone)]
pub struct AudioClip {
    data: Arc<Vec<f32>>,
    sample_rate: u32,
}

impl AudioClip {
    pub fn new(data: Vec<f32>, sample_rate: u32) -> Result<Self> {
        if data.len() % 2 != 0 {
            bail!("Audio data length must be even (Stereo Interleaved)");
        }
        Ok(Self {
            data: Arc::new(data),
            sample_rate,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn frame_count(&self) -> usize {
        self.data.len() / 2
    }

    pub fn duration(&self) -> f64 {
        self.frame_count() as f64 / self.sample_rate as f64
    }

    // Helper to access data if needed later
    pub fn data(&self) -> &Arc<Vec<f32>> {
        &self.data
    }
}
