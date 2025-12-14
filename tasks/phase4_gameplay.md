Task: Phase 4 Chart Management & Judgment (Oxidizer)

You are an expert Rust developer working on "Oxidizer".
Current Phase: Phase 4.
Goal: Implement the Chart Data Structure, Judgment Logic, and integrate them into the Bevy game loop.

Context:

Phase 1-3 are complete. We have Audio/Input core, Backend, and a synchronized Visual loop.

We need to turn the "falling rectangle demo" into a playable game logic.

Headless Constraint: Continue to rely on cargo check / cargo build for verification.

Step 1: Chart Data Structure

Objective: Define the internal representation of a Chart (independent of BMS format).

Create src/chart/mod.rs:

Define Judgment enum: Perfect, Great, Good, Bad, Poor, Miss.

Define Note struct:

pub struct Note {
    pub time: f64,
    pub lane: usize, // 1-7, Scratch=0
    pub kind: NoteKind, // Normal, Charge...
    pub sound_id: Option<usize>, // ID for audio clip
}


Define Chart struct:

pub struct Chart {
    pub notes: Vec<Note>,
    pub bpm_changes: Vec<BpmChange>,
    // ... metadata
}


Helper: Implement a function Chart::dummy() that returns a hardcoded chart (e.g., simple 4-beat rhythm) for testing.

Step 2: Judgment Logic (Pure Rust)

Objective: Implement the math for hit detection.

Create src/gameplay/judge.rs:

Struct JudgeMachine:

Holds state of active notes (which note is next for each lane).

Configurable timing windows (e.g., Perfect = +/- 16.6ms).

Method process_input(&mut self, event: InputEvent, chart: &Chart) -> Option<JudgmentResult>:

Find the nearest unprocessed note in the event's lane.

Calculate time difference delta = event.timestamp - note.time.

Determine Judgment based on delta.

Return result if handled.

Method check_misses(&mut self, current_time: f64, chart: &Chart) -> Vec<JudgmentResult>:

Detect notes that passed the timing window without input (POOR).

TEST FIRST: Create tests/judge_logic_test.rs.

Create a dummy chart.

Simulate input at perfect time -> Expect Perfect.

Simulate input late -> Expect Good or Bad.

Simulate no input -> Expect Miss (Poor).

Step 3: Bevy Integration

Objective: Connect the logic to the Bevy ECS.

Setup Resources:

Insert Chart (using Chart::dummy()) and JudgeMachine as Resources in main.rs.

Create Score resource to track hit counts.

Input Consumption System:

System: judgment_system.

Read from InputQueue (from Phase 2).

Call JudgeMachine::process_input.

Update Score based on result.

Log the judgment to console for verification.

Miss Detection System:

System: miss_detection_system.

Call JudgeMachine::check_misses using Conductor time.

Visual Update:

Modify move_notes (from Phase 3) to despawn or hide notes that have been judged (hit or missed).

Hint: Add a processed component or flag to visual entities.

Action:
Please execute Step 1 first. Then Step 2 (Logic Test). Finally Step 3 (Bevy Integration).