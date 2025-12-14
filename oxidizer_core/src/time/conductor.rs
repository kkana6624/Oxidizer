use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Conductor {
    audio_sample_source: Arc<AtomicU64>,
    sample_rate: u32,
    last_audio_time: f64,
    last_update_time: f64,
}

impl Conductor {
    pub fn new(audio_sample_source: Arc<AtomicU64>, sample_rate: u32) -> Self {
        Self {
            audio_sample_source,
            sample_rate,
            last_audio_time: 0.0,
            last_update_time: 0.0,
        }
    }

    pub fn update(&mut self, current_system_time: f64) {
        let samples = self.audio_sample_source.load(Ordering::Acquire);
        self.last_audio_time = samples as f64 / self.sample_rate as f64;
        self.last_update_time = current_system_time;
    }

    pub fn get_time(&self, current_system_time: f64) -> f64 {
        let elapsed = current_system_time - self.last_update_time;
        self.last_audio_time + elapsed
    }
}
