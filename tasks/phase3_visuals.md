Task: Phase 3 Visuals & Game Loop (Oxidizer)

You are an expert Rust developer working on "Oxidizer".
Current Phase: Phase 3.
Goal: Integrate the Bevy game engine, set up the main game loop, and synchronize visual objects with the Conductor (Audio Time).

Context:

Phase 1 & 2 are complete. Core logic, Input, and Audio backend are ready.

We are building a high-performance rhythm game interface.

Step 1: Bevy Setup & Window Configuration

Objective: Initialize Bevy with settings optimized for low latency and high refresh rates, designed to scale up to 4K resolution.

Update Cargo.toml: Add bevy (latest stable, e.g., 0.13 or 0.14).

Optimization: Disable unused Bevy features to speed up compile time (e.g., default-features = false, enable bevy_winit, bevy_core_pipeline, bevy_sprite, bevy_text, bevy_ui).

Create src/main.rs (or update existing):

Initialize App.

Configure WindowPlugin:

Title: "Oxidizer"

PresentMode: PresentMode::Mailbox (Crucial for low latency VSync on Linux/Windows).

Resolution: 1920x1080 (default for windowed testing), but ensure the architecture can handle up to 3840x2160 (4K).

High-DPI Support: Ensure the window respects the OS scale factor.

Run: Verify a blank window opens and runs smoothly.

Step 2: Integrate Conductor as a Resource

Objective: Make the Conductor accessible within Bevy systems to drive animations.

Resource Setup:

In main.rs, create the AudioStream and Conductor before building the App.

Insert Conductor into the App as a Resource (app.insert_resource(...)).

Note: You might need to wrap Conductor in a tuple struct or Arc<Mutex<...>> if it's not thread-safe for Bevy resources, but since it uses Atomics, it should be fine. If Bevy requires Resource to be Sync, ensure Conductor implements it.

Keep AudioStream alive (store it in a variable in main, or insert it as a Non-Send resource so it doesn't get dropped).

Debug Text System:

Create a system update_time_display that queries Res<Conductor> and updates a Text component on screen showing the current get_time().

Verify the numbers count up smoothly when the app runs.

Step 3: Visual Sync Test (Falling Note)

Objective: Visualize the "Audio is God" synchronization with resolution-independent coordinates.

Spawn a Note:

Create a startup system that spawns a Sprite (a simple white square) representing a Note.

Component: struct TestNote { target_time: f64 }. Set target_time to 2.0 seconds.

Movement System:

Create a system move_notes(conductor: Res<Conductor>, mut query: Query<(&TestNote, &mut Transform)>).

Logic (Resolution Independent):

Define a logical vertical workspace (e.g., LOGICAL_HEIGHT = 1080.0).

current_time = conductor.get_time()

Calculate position based on the logical height, not raw pixels, so it scales correctly on 4K.

y_position = (note.target_time - current_time) * (LOGICAL_HEIGHT * SPEED_FACTOR)

Update transform.translation.y.

Observation:

The note should fall smoothly.

Even if the window is dragged (simulating lag), the note's position should "snap" to the correct audio time, proving the Conductor is authoritative.

Action:
Please execute Step 1 first. Ensure Bevy compiles and opens a window. Then proceed to Step 2 and Step 3.