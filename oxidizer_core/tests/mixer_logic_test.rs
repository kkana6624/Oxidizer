use oxidizer_core::audio::assets::AudioClip;
use oxidizer_core::audio::mixer::AudioMixer;

#[test]
fn test_mixer_timing_offset() {
    let sample_rate = 44100;
    let (mut mixer, handle) = AudioMixer::new(sample_rate);

    // Create a clip with all 1.0s. 100 frames (200 samples)
    let data = vec![1.0; 200];
    let clip = AudioClip::new(data, sample_rate).expect("Failed to create clip");

    // Play at offset 100 frames.
    // 100 frames / 44100 Hz = 100.0/44100.0 seconds.
    let start_time = 100.0 / sample_rate as f64;
    handle.play(clip, start_time, 1.0); // 1.0 volume

    // Buffer of 512 frames (1024 samples)
    let mut buffer = vec![0.0; 1024];

    // Process at time 0.0
    mixer.process_buffer(&mut buffer, 0.0);

    // Frames 0-99 (indices 0-199) should be 0.0 (Silence before start)
    for i in 0..200 {
        assert_eq!(buffer[i], 0.0, "Buffer at index {} should be silent", i);
    }

    // Frames 100-199 (indices 200-399) should be 1.0 (The clip playing)
    for i in 200..400 {
        assert_eq!(buffer[i], 1.0, "Buffer at index {} should be 1.0", i);
    }

    // Frames 200+ (indices 400+) should be back to 0.0 (Clip finished)
     for i in 400..1024 {
        assert_eq!(buffer[i], 0.0, "Buffer at index {} should be silent after clip", i);
    }
}

#[test]
fn test_mixer_summing() {
    let sample_rate = 44100;
    let (mut mixer, handle) = AudioMixer::new(sample_rate);

    let data = vec![0.5; 200];
    let clip = AudioClip::new(data, sample_rate).expect("Failed to create clip");

    // Play two sounds at same time 0.0
    handle.play(clip.clone(), 0.0, 1.0);
    handle.play(clip.clone(), 0.0, 1.0);

    let mut buffer = vec![0.0; 1024];
    mixer.process_buffer(&mut buffer, 0.0);

    // Frames 0-99 (indices 0-199) should be 0.5 + 0.5 = 1.0
    for i in 0..200 {
        assert_eq!(buffer[i], 1.0, "Buffer at index {} should be summed to 1.0", i);
    }
}

#[test]
fn test_mixer_volume() {
    let sample_rate = 44100;
    let (mut mixer, handle) = AudioMixer::new(sample_rate);

    let data = vec![1.0; 200];
    let clip = AudioClip::new(data, sample_rate).expect("Failed to create clip");

    handle.play(clip, 0.0, 0.5); // Volume 0.5

    let mut buffer = vec![0.0; 1024];
    mixer.process_buffer(&mut buffer, 0.0);

    for i in 0..200 {
        assert_eq!(buffer[i], 0.5, "Buffer at index {} should be 0.5", i);
    }
}
