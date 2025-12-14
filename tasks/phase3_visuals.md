Task: Phase 3 Visuals & Game Loop (Oxidizer)

You are an expert Rust developer working on "Oxidizer".
Current Phase: Phase 3.
Goal: Integrate the Bevy game engine, set up the main game loop, and synchronize visual objects with the Conductor (Audio Time).

Context:

Phase 1 & 2 are complete. Core logic, Input, and Audio backend are ready.

We are building a high-performance rhythm game interface.

IMPORTANT: You are working in a headless environment. You cannot open windows or run GUI applications. Focus on implementing the code and verifying compilation.

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

Build Check: Run cargo check (or cargo build) to ensure dependencies are resolved and the code compiles.

Constraint: Do NOT try to cargo run. It will panic due to missing display server.

Step 2: Integrate Conductor as a Resource

Objective: Make the Conductor accessible within Bevy systems to drive animations.

Resource Setup:

In main.rs, create the AudioStream and Conductor before building the App.

Insert Conductor into the App as a Resource (app.insert_resource(...)).

Note: You might need to wrap Conductor in a tuple struct or Arc<Mutex<...>> if it's not thread-safe for Bevy resources, but since it uses Atomics, it should be fine. If Bevy requires Resource to be Sync, ensure Conductor implements it.

Keep AudioStream alive (store it in a variable in main, or insert it as a Non-Send resource so it doesn't get dropped).

Conductor Update System:

Create a system update_conductor_system that runs at the start of the frame (e.g., in PreUpdate).

Logic: Call conductor.update(time.elapsed_seconds_f64()) using Res<Time<Real>>.

Debug Text System:

Create a system update_time_display that queries Res<Conductor> and updates a Text component on screen showing the current get_time().

Compilation Verification: Ensure the code logic for querying resources compiles correctly.

Step 3: Visual Sync Test (IIDX-Style Scroll Logic)

Objective: Visualize the "Audio is God" synchronization using a robust scrolling model compatible with "Green Number" and "SUD+".

Define Scroll Configuration:

Create a Resource struct ScrollConfig.

Fields:

green_number: f32 (Target visibility time in milliseconds, e.g., 300.0).

sud_plus: f32 (Lane cover height in logical pixels, e.g., 250.0).

lift: f32 (Judgment line offset in logical pixels, e.g., 100.0).

lane_height: f32 (Total logical lane height, e.g., 1000.0).

Insert this resource with default values (GN=300, SUD=0, Lift=0).

Spawn a Note:

Create a startup system that spawns a Sprite (a simple white square) representing a Note.

Component: struct TestNote { target_time: f64 }. Set target_time to 2.0 seconds.

Movement System:

Create a system move_notes(conductor: Res<Conductor>, config: Res<ScrollConfig>, mut query: Query<(&TestNote, &mut Transform)>).

Logic (Green Number based):

visible_height = config.lane_height - config.sud_plus - config.lift

pixels_per_sec = visible_height / (config.green_number / 1000.0)

current_time = conductor.get_time()

time_diff = note.target_time - current_time

y_position = config.lift + (time_diff * pixels_per_sec)

Optimization: If y_position > config.lane_height - config.sud_plus, the note is hidden (behind SUD+).

Update transform.translation.y.

Final Check: Ensure src/main.rs compiles without errors.

Action:
Please execute Step 1 first. Ensure Bevy compiles. Then proceed to Step 2 and Step 3.