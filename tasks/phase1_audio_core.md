Task: Phase 1 Audio Core Implementation (Oxidizer)

You are an expert Rust developer. Your goal is to implement the Core Logic for the "Oxidizer" (IIDX Practice Tool) based on the architecture documents.

Current Phase: Phase 1 (Logic Verification).
Constraint: Do NOT implement hardware I/O (no real audio output, no window creation) yet. We focus on Test-Driven Development (TDD) to ensure the logic is mathematically correct.

Step 0: Project Setup

Initialize a new Rust project oxidizer_core with the following Cargo.toml.

[package]
name = "oxidizer_core"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1.0"
thiserror = "1.0"
crossbeam-channel = "0.5"
atomic_float = "0.1"
parking_lot = "0.12"
glam = "0.27"


Step 1: Implement Audio Assets

Objective: Define safe memory storage for audio data.

Create src/audio/mod.rs and src/audio/assets.rs.

TEST FIRST: Create tests/audio_assets_test.rs.

Test creation of AudioClip with dummy data.

Verify sample_rate, frame_count, and duration() properties.

Verify Clone behavior shares the underlying data (using Arc).

IMPLEMENT: Implement AudioClip struct to pass the tests.

Must use Arc<Vec<f32>> for data storage.

Assume Stereo (2ch) interleaved format.

Step 2: Implement Sample-Accurate Mixer

Objective: Implement the math for mixing sounds at precise timestamps.

Create src/audio/mixer.rs.

TEST FIRST: Create tests/mixer_logic_test.rs.

Scenario A: Buffer size 512 samples. Request a sound to play at offset 100. Verify buffer[0..99] is silent and buffer[100] contains data.

Scenario B: Two sounds overlapping. Verify their values are summed.

IMPLEMENT: Implement AudioMixer struct.

Function: process_buffer(buffer: &mut [f32], current_time: f64, sample_rate: u32)

Logic: offset = (target_time - current_time) * sample_rate.

Restriction: Do not use Mutex inside the processing loop.

Step 3: Implement Conductor (Timekeeping)

Objective: Logic for synchronizing game time with audio samples.

Create src/time/mod.rs and src/time/conductor.rs.

TEST FIRST: Create tests/conductor_test.rs.

Simulate an AtomicF64 being updated by a fake audio thread.

Call conductor.update().

Verify conductor.get_time() returns the correct interpolated time.

IMPLEMENT: Conductor struct.

Action:
Please start with Step 1. Write the test file, then the implementation, and run cargo test.