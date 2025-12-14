use oxidizer_core::time::conductor::Conductor;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[test]
fn test_conductor_interpolation() {
    let sample_rate = 1000;
    let audio_samples = Arc::new(AtomicU64::new(0));
    let mut conductor = Conductor::new(audio_samples.clone(), sample_rate);

    // Initial state: System time 10.0
    // Audio samples: 0 -> time 0.0
    conductor.update(10.0);

    // Verify base time
    assert!((conductor.get_time(10.0) - 0.0).abs() < 1e-5);

    // Verify interpolation: 0.1s later in system time
    // Audio thread hasn't moved, but we extrapolate
    assert!((conductor.get_time(10.1) - 0.1).abs() < 1e-5);

    // Simulate audio thread advance
    // Audio advanced to 0.5s -> 500 samples
    audio_samples.store(500, Ordering::SeqCst);

    // Before update, conductor still extrapolates from old state (time 0.0, system 10.0)
    // System time 10.6 (0.6s elapsed since 10.0) -> time should be 0.6
    assert!((conductor.get_time(10.6) - 0.6).abs() < 1e-5);

    // Now update conductor
    // System time 10.6. Audio time 0.5.
    conductor.update(10.6);

    // Now get_time(10.6) should return audio_time (0.5)
    assert!((conductor.get_time(10.6) - 0.5).abs() < 1e-5);

    // And future
    assert!((conductor.get_time(10.7) - 0.6).abs() < 1e-5); // 0.5 + (10.7 - 10.6)
}
