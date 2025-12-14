use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioClipError {
    #[error("Audio data length must be even for stereo (2ch) format")]
    OddSampleCount,
}

#[derive(Debug, Clone)]
pub struct AudioClip {
    data: Arc<Vec<f32>>,
    sample_rate: u32,
    frame_count: usize,
}

impl AudioClip {
    /// Creates a new AudioClip.
    ///
    /// The data is assumed to be stereo (2ch) interleaved samples.
    /// Therefore, the length of the data vector must be even.
    pub fn new(data: Vec<f32>, sample_rate: u32) -> Result<Self, AudioClipError> {
        #[allow(clippy::manual_is_multiple_of)]
        if data.len() % 2 != 0 {
            return Err(AudioClipError::OddSampleCount);
        }

        let frame_count = data.len() / 2;
        Ok(Self {
            data: Arc::new(data),
            sample_rate,
            frame_count,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    pub fn duration(&self) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.frame_count as f64 / self.sample_rate as f64
    }

    pub fn samples(&self) -> &[f32] {
        &self.data
    }
}
