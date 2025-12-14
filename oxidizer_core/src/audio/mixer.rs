use crate::audio::assets::AudioClip;
use crossbeam_channel::{unbounded, Receiver, Sender};

struct Voice {
    clip: AudioClip,
    start_time: f64,
    volume: f32,
}

pub enum MixerCommand {
    Play {
        clip: AudioClip,
        start_time: f64,
        volume: f32,
    },
}

pub struct AudioMixer {
    sample_rate: u32,
    voices: Vec<Voice>,
    command_rx: Receiver<MixerCommand>,
}

#[derive(Clone)]
pub struct MixerHandle {
    command_tx: Sender<MixerCommand>,
}

impl MixerHandle {
    pub fn play(&self, clip: AudioClip, start_time: f64, volume: f32) {
        let _ = self.command_tx.send(MixerCommand::Play { clip, start_time, volume });
    }
}

impl AudioMixer {
    pub fn new(sample_rate: u32) -> (Self, MixerHandle) {
        let (tx, rx) = unbounded();
        (
            Self {
                sample_rate,
                voices: Vec::new(),
                command_rx: rx,
            },
            MixerHandle { command_tx: tx },
        )
    }

    pub fn active_voice_count(&self) -> usize {
        self.voices.len()
    }

    pub fn process_buffer(&mut self, buffer: &mut [f32], current_time: f64) {
        // 1. Process commands
        while let Ok(cmd) = self.command_rx.try_recv() {
            match cmd {
                MixerCommand::Play { clip, start_time, volume } => {
                    self.voices.push(Voice { clip, start_time, volume });
                }
            }
        }

        let frames_in_buffer = buffer.len() / 2; // Stereo
        let buffer_end_frame = frames_in_buffer as isize;

        // 2. Mix voices
        for voice in self.voices.iter() {
            let clip = &voice.clip;
            let clip_data = clip.data();
            let clip_frames = clip.frame_count() as isize;

            // Calculate start frame of the voice relative to the start of this buffer
            // time_since_start > 0 means voice started in the past (we are into the voice)
            let time_since_start = current_time - voice.start_time;
            let start_frame_offset = (time_since_start * self.sample_rate as f64).round() as isize;

            // We need to write to buffer[i] for i in 0..frames_in_buffer
            // The corresponding voice frame is (start_frame_offset + i)
            // We need 0 <= voice_frame < clip_frames

            // i >= -start_frame_offset
            let start_i = (-start_frame_offset).max(0);
            // i < clip_frames - start_frame_offset
            let end_i = (clip_frames - start_frame_offset).min(buffer_end_frame);

            if start_i < end_i {
                for i in start_i..end_i {
                    let voice_frame = (start_frame_offset + i) as usize;
                    let buf_idx = i as usize * 2;
                    let voice_idx = voice_frame * 2;

                    // Stereo sum
                    buffer[buf_idx] += clip_data[voice_idx] * voice.volume;
                    buffer[buf_idx + 1] += clip_data[voice_idx + 1] * voice.volume;
                }
            }
        }

        // 3. Cleanup finished voices
        // A voice is finished if we have passed its end.
        // i.e. start_frame_offset (relative to buffer start) is beyond clip length.
        // Actually, strictly speaking, if start_frame_offset >= clip_frames,
        // then even the first sample of the buffer (i=0) maps to voice_frame >= clip_frames.
        self.voices.retain(|v| {
             let time_since_start = current_time - v.start_time;
             let start_frame_offset = (time_since_start * self.sample_rate as f64).round() as isize;
             start_frame_offset < v.clip.frame_count() as isize
        });
    }
}
