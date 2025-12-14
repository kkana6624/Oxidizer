use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig, OutputCallbackInfo};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use crate::audio::mixer::AudioMixer;

pub struct AudioStream {
    _stream: Stream,
}

impl AudioStream {
    pub fn new(
        mixer: Arc<parking_lot::Mutex<AudioMixer>>,
        processed_samples: Arc<AtomicU64>,
    ) -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No default output device available"))?;

        // We want to force Stereo 44.1kHz
        let config = StreamConfig {
            channels: 2,
            sample_rate: cpal::SampleRate(44100),
            buffer_size: cpal::BufferSize::Default,
        };

        let sample_count_clone = processed_samples.clone();

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &OutputCallbackInfo| {
                // We need to lock the mixer to process buffer
                let mut mixer_guard = mixer.lock();

                // AudioMixer expects samples in interleaved stereo.
                // data.len() is the number of samples (frames * channels).
                // Our mixer handles processing in place.

                // Zero out the buffer first? Or does mixer accumulate?
                // Mixer documentation says: "buffer[buf_idx] += ..." so it accumulates.
                // We should zero it out before passing to mixer if mixer assumes clean slate or additive mixing.
                // Usually cpal buffer contains garbage or silence.
                // Let's assume we need to zero it if the mixer adds.
                for sample in data.iter_mut() {
                    *sample = 0.0;
                }

                // Current audio time in seconds.
                let current_samples = sample_count_clone.load(Ordering::Acquire);
                let current_time = current_samples as f64 / 44100.0;

                mixer_guard.process_buffer(data, current_time);

                // Update processed samples
                let written_frames = (data.len() / 2) as u64;
                sample_count_clone.fetch_add(written_frames, Ordering::Release);
            },
            move |err| {
                eprintln!("Audio output error: {}", err);
            },
            None, // timeout
        )?;

        stream.play()?;

        Ok(Self {
            _stream: stream,
        })
    }
}
