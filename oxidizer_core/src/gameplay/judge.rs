use crate::chart::{Chart, Judgment};
use crate::input::events::{Button, InputEvent};

#[derive(Debug, Clone, PartialEq)]
pub struct JudgmentResult {
    pub note_index: usize,
    pub judgment: Judgment,
    pub delta: f64,
}

pub struct JudgeMachine {
    /// Tracks the index of the next unprocessed note for each lane (0-7).
    pub next_note_index: [usize; 8],

    // Timing windows in seconds (half-width)
    pub perfect_window: f64,
    pub great_window: f64,
    pub good_window: f64,
    pub bad_window: f64,
}

impl Default for JudgeMachine {
    fn default() -> Self {
        Self {
            next_note_index: [0; 8],
            perfect_window: 0.016, // ~1 frame
            great_window: 0.040,
            good_window: 0.100,
            bad_window: 0.200, // Capture window
        }
    }
}

impl JudgeMachine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process_input(&mut self, event: InputEvent, chart: &Chart) -> Option<JudgmentResult> {
        if !event.pressed {
            return None; // We only judge on press for now
        }

        let lane = button_to_lane(event.button)?;
        let current_index = self.next_note_index[lane];

        // Iterate through notes in this lane starting from current_index
        // to find the first one that matches the lane.
        // Actually, the `next_note_index` array tracks the index into the GLOBAL chart.notes?
        // OR the index into a pre-filtered per-lane list?
        // The `Chart` struct has `notes: Vec<Note>`.
        // If we use global index, it's hard to track "next note in lane 1" vs "next note in lane 2".
        // The `next_note_index` implies we need to scan.
        //
        // OPTIMIZATION:
        // A naive scan every input is O(N).
        // A better approach is to pre-process the chart into lanes, or keep track of indices better.
        //
        // Given the phase constraints, I will do a scan but I will store the *global* index in `next_note_index`.
        // However, `next_note_index[lane]` implies we store the index *of the next note in that lane*.
        // But `chart.notes` is a single flat list.
        // So `next_note_index[lane]` should probably be the index in `chart.notes` where we start searching?
        // No, that's messy if notes are interleaved.
        //
        // PROPOSAL:
        // Let's assume `next_note_index[lane]` is the index into `chart.notes` of the next candidate for that lane.
        // But since `chart.notes` is sorted by time, we can't easily jump to the next note of a specific lane without scanning or auxiliary structures.
        //
        // Alternative: `next_note_index` is just a cursor into `chart.notes`? No, because we have 8 independent lanes.
        //
        // To do this efficiently without changing `Chart` structure (which is flat Vec<Note>):
        // We need to either:
        // 1. Scan from the *last known position* for that lane.
        // 2. Pre-calculate indices per lane.
        //
        // Let's rely on `next_note_index[lane]` being the index in `chart.notes`.
        // Initialize all to 0? No, that's wrong.
        //
        // Actually, if `chart.notes` is sorted by time, we can't maintain 8 separate indices into it easily unless they all move forward monotonically.
        //
        // Let's implement a helper `find_next_note(start_index, lane)`?
        // But we want to persist this.
        //
        // Let's change `next_note_index` to be `[usize; 8]` where the value is the index in `chart.notes`.
        // But wait, if I have Note 0 (Lane 1), Note 1 (Lane 2).
        // Lane 1 cursor is 0. Lane 2 cursor is 1.
        // Correct.
        //
        // So, `JudgeMachine` needs to be initialized by finding the first note for each lane.
        // Or we just search from `next_note_index[lane]`.
        // When we process a note at `idx`, we must advance `next_note_index[lane]` to the *next* note in that lane.

        // Find the note at `current_index`.
        // Check if it really is for this lane. (It should be, if we maintain invariants).
        // But initially, `default()` is all 0s. Note 0 might be Lane 1.
        // So `next_note_index[2]` being 0 is wrong if Note 0 is Lane 1.
        //
        // SOLUTION:
        // On `process_input` or `check_misses`, if `next_note_index[lane]` points to a note of a DIFFERENT lane,
        // we must advance it until we find a note of the correct lane or hit end.
        //
        // This effectively lazy-initializes/maintains the cursors.

        let mut idx = current_index;

        // Advance cursor to the next note for this lane
        while idx < chart.notes.len() {
            if chart.notes[idx].lane == lane {
                break;
            }
            idx += 1;
        }

        // Update the stored index to this found one (or len if none)
        // We can safely update this because all previous notes in this lane (if any) were processed/skipped.
        // Wait, if we just skipped notes of *other* lanes, that's fine.
        // But we shouldn't modify `self.next_note_index[lane]` permanently here just by peeking?
        // Actually we can, because any previous notes in `chart.notes` *for this lane* must have been processed already (otherwise `current_index` would be lower).
        // Notes for *other* lanes are irrelevant to `next_note_index[lane]`.
        self.next_note_index[lane] = idx;

        if idx >= chart.notes.len() {
            return None;
        }

        let note = &chart.notes[idx];
        let delta = event.timestamp - note.time;
        let abs_delta = delta.abs();

        if abs_delta <= self.bad_window {
            // Hit!
            let judgment = if abs_delta <= self.perfect_window {
                Judgment::Perfect
            } else if abs_delta <= self.great_window {
                Judgment::Great
            } else if abs_delta <= self.good_window {
                Judgment::Good
            } else {
                Judgment::Bad
            };

            // Advance the cursor for this lane
            self.next_note_index[lane] = idx + 1;
            // We also need to advance to the next actual note for this lane to be ready?
            // We can do it lazily next time.

            Some(JudgmentResult {
                note_index: idx,
                judgment,
                delta,
            })
        } else {
            // Outside window.
            // If delta is negative (input < note), we are too early. Ignore.
            // If delta is positive (input > note), we are too late.
            // But if we are too late, `check_misses` should have caught it?
            // Or maybe `check_misses` hasn't run yet.
            // If we are way too late, we probably shouldn't trigger a "Bad" hit, we should let it be a Miss.
            // But usually "Bad" window is the cutoff.
            None
        }
    }

    pub fn check_misses(&mut self, current_time: f64, chart: &Chart) -> Vec<JudgmentResult> {
        let mut results = Vec::new();

        for lane in 0..8 {
            loop {
                let mut idx = self.next_note_index[lane];

                // Advance to next valid note for this lane
                while idx < chart.notes.len() {
                    if chart.notes[idx].lane == lane {
                        break;
                    }
                    idx += 1;
                }
                self.next_note_index[lane] = idx; // Update cursor

                if idx >= chart.notes.len() {
                    break;
                }

                let note = &chart.notes[idx];
                // If current_time > note.time + bad_window, it's a miss
                if current_time > note.time + self.bad_window {
                    results.push(JudgmentResult {
                        note_index: idx,
                        judgment: Judgment::Poor,
                        delta: current_time - note.time, // Large positive delta
                    });

                    // Advance past this missed note
                    self.next_note_index[lane] = idx + 1;
                    // Continue loop to check for more misses in this lane
                } else {
                    // Not missed yet
                    break;
                }
            }
        }
        results
    }
}

pub fn button_to_lane(button: Button) -> Option<usize> {
    match button {
        Button::ScratchClockwise | Button::ScratchCounterClockwise => Some(0),
        Button::Key1 => Some(1),
        Button::Key2 => Some(2),
        Button::Key3 => Some(3),
        Button::Key4 => Some(4),
        Button::Key5 => Some(5),
        Button::Key6 => Some(6),
        Button::Key7 => Some(7),
        _ => None,
    }
}
