use oxidizer_core::audio::assets::AudioClip;
use oxidizer_core::audio::backend::AudioStream;
use oxidizer_core::audio::mixer::AudioMixer;
use oxidizer_core::time::conductor::Conductor;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use parking_lot::Mutex;

fn main() -> anyhow::Result<()> {
    println!("Initializing Audio System...");

    let sample_rate = 44100;

    // Create shared components
    let (mixer, mixer_handle) = AudioMixer::new(sample_rate);
    let mixer = Arc::new(Mutex::new(mixer));
    let processed_samples = Arc::new(AtomicU64::new(0));

    // Initialize Conductor
    let mut conductor = Conductor::new(processed_samples.clone(), sample_rate);

    // Initialize Audio Backend
    println!("Starting AudioStream...");
    let _stream = AudioStream::new(mixer.clone(), processed_samples.clone())?;

    // Create a dummy sine wave sound
    println!("Generating sine wave...");
    let mut sine_data = Vec::new();
    let duration_secs = 2.0;
    let frames = (duration_secs * sample_rate as f64) as usize;
    let freq = 440.0;

    for i in 0..frames {
        let t = i as f64 / sample_rate as f64;
        let sample = (t * freq * 2.0 * std::f64::consts::PI).sin() as f32;
        // Stereo interleaved
        sine_data.push(sample * 0.5); // Left
        sine_data.push(sample * 0.5); // Right
    }

    let clip = AudioClip::new(sine_data, sample_rate)?;

    // Play the sound
    println!("Playing sound...");
    mixer_handle.play(clip, 0.5, 1.0); // Start at 0.5s audio time

    // Simulation loop
    let start_time = std::time::Instant::now();
    loop {
        let elapsed = start_time.elapsed().as_secs_f64();
        if elapsed > 4.0 {
            break;
        }

        conductor.update(elapsed);
        let audio_time = conductor.get_time(elapsed);

        println!("System Time: {:.2} | Audio Time: {:.2}", elapsed, audio_time);
        thread::sleep(Duration::from_millis(100));
    }

    println!("Done.");
    Ok(())
}
