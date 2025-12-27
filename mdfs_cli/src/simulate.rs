use std::collections::BTreeSet;

use mdf_schema::{MdfChart, NoteKind};

pub fn run_simulation(chart: &MdfChart) -> anyhow::Result<()> {
    // Collect all interesting time points
    let mut time_points = BTreeSet::new();

    // Notes
    for note in &chart.notes {
        time_points.insert(note.time_us);
        if let Some(end) = note.kind.end_time_us() {
            time_points.insert(end);
        }
        if let NoteKind::MultiSpinScratch {
            reverse_checkpoints_us,
            ..
        }
        | NoteKind::HellMultiSpinScratch {
            reverse_checkpoints_us,
            ..
        } = &note.kind
        {
            for &t in reverse_checkpoints_us {
                time_points.insert(t);
            }
        }
    }

    // BPM changes
    for ve in &chart.visual_events {
        time_points.insert(ve.time_us);
    }

    // BGM (optional, but good for context)
    for be in &chart.bgm_events {
        time_points.insert(be.time_us);
    }

    if time_points.is_empty() {
        println!("Chart is empty.");
        return Ok(());
    }

    println!("Simulation Start ({} us total)", chart.meta.total_duration_us);
    println!("Time(us) | S 1 2 3 4 5 6 7 | Info");
    println!("---------|-----------------|------------------");

    // Track holding state per lane: None or Some(HoldChar)
    // For MSS/BSS/CN, we want to show '|' or specific chars
    let mut holding: [Option<char>; 8] = [None; 8];

    // Track active note objects to determine holding state logic
    // We need to know "what note is currently holding on this lane" to decide when to clear it
    // Or just re-evaluate state at each step?
    // Since notes are sorted, we can just process events at exactly `t`.

    for &t in &time_points {
        let mut info_parts = Vec::new();

        // 1. Process BPM changes first
        for ve in &chart.visual_events {
            if ve.time_us == t {
                let current_bpm = ve.bpm;
                info_parts.push(format!("BPM: {:.1}", current_bpm));
            }
        }

        // 2. Determine Lane Strings
        // We need to know what happens at exactly `t`.
        // - Start of Note -> Show char
        // - End of Note -> Show 'E' (or Tail char)
        // - Checkpoint -> Show '!'
        // - Holding -> Show '|'
        // - Empty -> '.'

        // First, check for Starts and Ends at this `t`
        let mut lane_chars = ['.'; 8];

        // Populate "Holding" state from previous iteration (persisted in `holding`)
        // But for the *current* line output, if we are holding, we default to '|' unless overridden by an event.
        for i in 0..8 {
            if holding[i].is_some() {
                lane_chars[i] = '|';
            }
        }

        // Process Note Events at `t`
        for note in &chart.notes {
            // Start
            if note.time_us == t {
                let ch = match note.kind {
                    NoteKind::Tap => 'N',
                    NoteKind::ChargeNote { .. } => 'C',
                    NoteKind::HellChargeNote { .. } => 'H',
                    NoteKind::BackSpinScratch { .. } => 'B',
                    NoteKind::HellBackSpinScratch { .. } => 'b',
                    NoteKind::MultiSpinScratch { .. } => 'M',
                    NoteKind::HellMultiSpinScratch { .. } => 'm',
                };
                lane_chars[note.col as usize] = ch;

                // If it's a hold, start holding
                if note.kind.end_time_us().is_some() {
                     holding[note.col as usize] = Some(ch);
                }
            }

            // End
            if let Some(end) = note.kind.end_time_us() {
                if end == t {
                    // It ends here.
                    // Visual choice: Show 'E' or just the tail char?
                    // User example had "S ... S" for toggles.
                    // Let's use ']' or matching end char.
                    // Or maybe just the same char as start to indicate "Event here".
                    // But typically visualizers show "Head" ... "Tail".
                    lane_chars[note.col as usize] = '#'; // Tail marker
                    holding[note.col as usize] = None;
                }
            }

            // Checkpoints (MSS)
            if let NoteKind::MultiSpinScratch { reverse_checkpoints_us, .. } | NoteKind::HellMultiSpinScratch { reverse_checkpoints_us, .. } = &note.kind {
                 for &cp in reverse_checkpoints_us {
                     if cp == t {
                         lane_chars[note.col as usize] = '!';
                     }
                 }
            }
        }

        // BGM info
        let mut bgm_count = 0;
        for bgm in &chart.bgm_events {
            if bgm.time_us == t {
                bgm_count += 1;
            }
        }
        if bgm_count > 0 {
             info_parts.push(format!("BGM x{}", bgm_count));
        }

        // Format Lane String
        let lane_str: String = lane_chars.iter().map(|c| format!("{} ", c)).collect::<String>().trim_end().to_string();

        // Print
        println!("{:8} | {} | {}", t, lane_str, info_parts.join(", "));
    }

    Ok(())
}
