Task: Phase 5 BMS Loading & Audio Triggering (Oxidizer)

You are an expert Rust developer working on "Oxidizer".
Current Phase: Phase 5.
Goal: Implement BMS File Parsing, Audio Asset Loading, and integrate Key Sound Triggering into the gameplay loop.

Context:

Phase 1-4 are complete. We have a running game loop with dummy charts and valid judgment logic.

We now need to load real data and play real sounds.

Headless Constraint: Continue to rely on cargo check / cargo build.

Step 1: BMS Parsing Logic

Objective: Parse .bms/.bme files and convert them into our internal Chart structure.

Update Cargo.toml: Add bms crate (or a similar robust parser like bms-rs if bms is outdated). Also add encoding_rs to handle Shift-JIS (common in BMS).

Create src/chart/loader.rs:

Implement BmsLoader struct.

Function load_bms(path: &Path) -> Result<(Chart, HashMap<usize, String>)>:

Chart: The internal gameplay chart.

HashMap<usize, String>: Map of WAV ID (e.g., 01, 0A) to filename.

Conversion Logic:

Map BMS channels (11-17, 21-29, etc.) to Oxidizer lanes.

Extract BPM changes (channel 03, 08).

Handle "Background Notes" (Channel 01) by creating Note with lane: 0 (or a special BGM constant) and kind: NoteKind::Bgm.

TEST FIRST: Create tests/bms_loader_test.rs.

Create a minimal temporary BMS file string.

Parse it and verify the Chart contains the correct notes and BPMs.

Step 2: Audio Asset Loading (Real Implementation)

Objective: Implement actual file loading for AudioClip.

Update Cargo.toml: Add symphonia (with all features) for robust audio decoding (WAV, OGG, MP3).

Update src/audio/assets.rs:

Implement AudioClip::load_file(path: &Path) -> Result<Self>.

Use symphonia to decode the file into the required Vec<f32> (Stereo Interleaved) format.

Handle resampling if the source is not 44100Hz (simple linear interpolation is fine for now, or ensure AudioMixer handles rate mismatch later. For Phase 5, assume source matches or implement naive resampling).

Create src/audio/manager.rs:

Struct AudioManager.

Method load_samples(root: &Path, wav_map: HashMap<usize, String>) -> HashMap<usize, Arc<AudioClip>>.

Iterate through the map, load files, and store them.

Step 3: Sound Triggering Integration

Objective: Play sounds on judgment and auto-play BGM.

Setup Resources:

In main.rs, update startup logic to load a specific BMS file (hardcoded path or CLI arg) if available, otherwise fallback to Dummy.

Insert AudioManager (or the loaded HashMap<usize, Arc<AudioClip>>) as a Resource.

Update judgment_system:

When a judgment is Perfect, Great, or Good:

Retrieve sound_id from the Note.

If sound_id exists, look up the AudioClip in the resource.

Call mixer_handle.play_one_shot(clip).

Create bgm_system:

Query for Notes with kind: NoteKind::Bgm.

Check conductor.get_time().

If note.time is passed and not yet processed:

Trigger the sound via mixer_handle.

Mark note as processed.

Action:
Please execute Step 1 first. Then Step 2. Finally Step 3.