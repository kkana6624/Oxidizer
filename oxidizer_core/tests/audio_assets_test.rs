use oxidizer_core::audio::assets::AudioClip;

#[test]
fn test_audio_clip_creation() {
    let sample_rate = 44100;
    // 1 second of silence, stereo (44100 frames * 2 channels)
    let samples = vec![0.0; 44100 * 2];
    let clip = AudioClip::new(samples.clone(), sample_rate).unwrap();

    assert_eq!(clip.sample_rate(), sample_rate);
    assert_eq!(clip.frame_count(), 44100);
    // Use an epsilon for floating point comparison, although precise division should give 1.0 here
    assert!((clip.duration() - 1.0).abs() < f64::EPSILON);

    // Test data access?
    // Not explicitly required by the test description but good to check content
}

#[test]
fn test_audio_clip_clone_sharing() {
    let samples = vec![0.0; 100]; // 50 frames
    let clip1 = AudioClip::new(samples, 44100).unwrap();
    let clip2 = clip1.clone();

    // Verify properties match
    assert_eq!(clip1.sample_rate(), clip2.sample_rate());

    // Verify data sharing
    // Since we are black-boxing, we might not be able to check pointer unless we expose it.
    // However, we can check if they point to the same memory if we expose a way to get the slice.
    // Let's assume AudioClip has a method `samples(&self) -> &[f32]`.

    assert_eq!(clip1.samples().as_ptr(), clip2.samples().as_ptr());
}

#[test]
fn test_audio_clip_invalid_creation() {
    let sample_rate = 44100;
    // Odd number of samples, should fail for stereo
    let samples = vec![0.0; 101];
    let result = AudioClip::new(samples, sample_rate);
    assert!(result.is_err());
}
