Task: Phase 2 Input & Hardware Integration (Oxidizer)

You are an expert Rust developer working on "Oxidizer".
Current Phase: Phase 2.
Goal: Implement the Input System core logic and integrate the Audio System with actual hardware (cpal).

Context:

Phase 1 is complete (AudioClip, AudioMixer, Conductor are implemented and tested).

We are now moving from pure logic to integration.

Step 1: Input System Core

Objective: Define input events and a thread-safe queue mechanism.

Update Cargo.toml: Add gilrs (for gamepad/controller) and crossbeam-channel.

Create src/input/mod.rs & src/input/events.rs:

Define InputEvent struct:

pub struct InputEvent {
    pub timestamp: f64, // Absolute audio time
    pub button: Button, // Enum (Key1..7, Scratch, Start...)
    pub pressed: bool,
}


Define InputQueue struct that wraps a crossbeam_channel::Receiver.

TEST FIRST: Create tests/input_queue_test.rs.

Test sending events from a producer thread and receiving them in a consumer thread.

Verify timestamps are preserved.

IMPLEMENT: Implement the structs.

Step 2: Audio Backend Integration (cpal)

Objective: Connect AudioMixer to the OS audio driver using cpal.

Update Cargo.toml: Add cpal.

Create src/audio/backend.rs:

Implement AudioStream struct that initializes cpal.

Crucial: In the cpal data callback, you must call AudioMixer::process_buffer.

Sync: Update the AtomicU64 (processed samples) inside the callback for the Conductor.

Integration Test (CLI): Create examples/audio_check.rs.

This is a runnable binary, not a unit test.

Initialize AudioStream.

Load a dummy sound (e.g., generate a sine wave AudioClip).

Send a Play command to the mixer.

Keep the main thread alive for a few seconds to let the sound play.

Action:
Please execute Step 1 first. Once the input logic is tested and green, proceed to Step 2.