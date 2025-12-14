use std::sync::Arc;
use atomic_float::AtomicF64;
use std::sync::atomic::Ordering;

pub struct Conductor {
    audio_time_source: Arc<AtomicF64>,
    last_audio_time: f64,
    last_update_time: f64,
}

impl Conductor {
    pub fn new(audio_time_source: Arc<AtomicF64>) -> Self {
        Self {
            audio_time_source,
            last_audio_time: 0.0,
            last_update_time: 0.0,
        }
    }

    pub fn update(&mut self, current_system_time: f64) {
        self.last_audio_time = self.audio_time_source.load(Ordering::Acquire);
        self.last_update_time = current_system_time;
    }

    pub fn get_time(&self, current_system_time: f64) -> f64 {
        let elapsed = current_system_time - self.last_update_time;
        self.last_audio_time + elapsed
    }
}
