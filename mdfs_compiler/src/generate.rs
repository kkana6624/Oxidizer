use std::collections::{HashMap, HashSet};

use mdf_schema::{BgmEvent, Microseconds, Note, NoteKind};

use crate::CompileError;
use crate::parser::{RevSpec, SoundSpec, TrackLine};

#[derive(Debug, Clone)]
enum OpenHoldKind {
    Charge,
    HellCharge,
    Bss,
    HellBss,
    Mss { rev: RevSpec },
    HellMss { rev: RevSpec },
}

#[derive(Debug, Clone)]
struct OpenHold {
    start_line: usize,
    start_time_us: Microseconds,
    start_step_index: usize,
    sound_id: Option<String>,
    kind: OpenHoldKind,
    marker_checkpoints_us: Vec<Microseconds>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StartKind {
    Tap,
    HoldStart,
}

fn register_tap_start(
    start_kinds: &mut HashMap<(Microseconds, u8), StartKind>,
    time_us: Microseconds,
    lane_u8: u8,
    lane_for_message: usize,
    step_index: usize,
    line: usize,
) -> Result<(), CompileError> {
    if let Some(existing) = start_kinds.get(&(time_us, lane_u8)) {
        if *existing == StartKind::HoldStart {
            return Err(
                CompileError::new(
                    "E4004",
                    format!(
                        "tap overlaps hold start at same (time_us,lane) (time_us={time_us}, lane={lane_for_message})"
                    ),
                    line,
                )
                .with_step_index(step_index)
                .with_time_us(time_us)
                .with_lane(lane_u8),
            );
        }
        return Ok(());
    }

    start_kinds.insert((time_us, lane_u8), StartKind::Tap);
    Ok(())
}

fn register_hold_start(
    start_kinds: &mut HashMap<(Microseconds, u8), StartKind>,
    time_us: Microseconds,
    lane_u8: u8,
    lane_for_message: usize,
    step_index: usize,
    line: usize,
) -> Result<(), CompileError> {
    if let Some(existing) = start_kinds.get(&(time_us, lane_u8)) {
        if *existing == StartKind::Tap {
            return Err(
                CompileError::new(
                    "E4004",
                    format!(
                        "hold start overlaps tap at same (time_us,lane) (time_us={time_us}, lane={lane_for_message})"
                    ),
                    line,
                )
                .with_step_index(step_index)
                .with_time_us(time_us)
                .with_lane(lane_u8),
            );
        }
        return Ok(());
    }

    start_kinds.insert((time_us, lane_u8), StartKind::HoldStart);
    Ok(())
}

fn handle_marker_checkpoint(
    open: &mut [Option<OpenHold>],
    bgm_events: &mut Vec<BgmEvent>,
    time_us: Microseconds,
    step_index: usize,
    sound: &SoundSpec,
    resources: &HashMap<String, String>,
    line: usize,
) -> Result<(), CompileError> {
    // marker checkpoint only valid inside MSS/HMSS hold
    let Some(open0) = &mut open[0] else {
        return Err(
            CompileError::new(
                "E4003",
                "'!' is only valid while MSS/HMSS is active",
                line,
            )
            .with_step_index(step_index)
            .with_time_us(time_us)
            .with_lane(0),
        );
    };

    match open0.kind {
        OpenHoldKind::Mss { .. } | OpenHoldKind::HellMss { .. } => {
            open0.marker_checkpoints_us.push(time_us);
            push_bgm_events_from_sound(bgm_events, time_us, sound, resources, line)
        }
        OpenHoldKind::Bss | OpenHoldKind::HellBss => Err(
            CompileError::new(
                "E4102",
                "'!' is not allowed while BSS/HBSS is active",
                line,
            )
            .with_step_index(step_index)
            .with_time_us(time_us)
            .with_lane(0),
        ),
        _ => Err(
            CompileError::new(
                "E4003",
                "'!' is only valid while MSS/HMSS is active",
                line,
            )
            .with_step_index(step_index)
            .with_time_us(time_us)
            .with_lane(0),
        ),
    }
}

pub(crate) fn pass2_generate(
    track: &[TrackLine],
    step_times: &[Microseconds],
    resources: &HashMap<String, String>,
) -> Result<(Vec<Note>, Vec<BgmEvent>), CompileError> {
    let mut notes = Vec::new();
    let mut bgm_events = Vec::new();
    let mut start_kinds: HashMap<(Microseconds, u8), StartKind> = HashMap::new();

    let mut open: Vec<Option<OpenHold>> = vec![None; 8];
    let mut step_index = 0usize;

    for line in track {
        match line {
            TrackLine::Directive { .. } => {}
            TrackLine::Step {
                line,
                cells,
                sound,
                rev,
            } => {
                let time_us = step_times
                    .get(step_index)
                    .copied()
                    .ok_or_else(|| CompileError::new("E1101", "internal step index mismatch", *line))?;

                let lane_sounds = lane_sounds(sound);
                let has_any_note = cells.iter().any(|c| !matches!(c, '.'));

                // If step has only '.' but has SOUND_SPEC, generate BGM events (optional feature in spec)
                if !has_any_note {
                    push_bgm_events_from_sound(&mut bgm_events, time_us, sound, resources, *line)?;
                }

                // Validate @rev directives appear only on MSS/HMSS start lines.
                if (rev.every.is_some() || !rev.at.is_empty()) && !matches!(cells[0], 'm' | 'M') {
                    return Err(
                        CompileError::new(
                            "E4201",
                            "@rev_every/@rev_at only allowed on MSS/HMSS start line",
                            *line,
                        )
                        .with_step_index(step_index)
                        .with_time_us(time_us),
                    );
                }

                for col in 0..8 {
                    let ch = cells[col];
                    match ch {
                        '.' => {}
                        'N' | 'S' => {
                            if let Some(id) = lane_sounds[col].as_deref() {
                                validate_sound_id(resources, id, *line, Some(col))?;
                            }

                            let lane_u8 = col as u8;
                            register_tap_start(
                                &mut start_kinds,
                                time_us,
                                lane_u8,
                                col,
                                step_index,
                                *line,
                            )?;

                            notes.push(Note {
                                time_us,
                                col: col as u8,
                                kind: NoteKind::Tap,
                                sound_id: lane_sounds[col].clone(),
                            });
                        }
                        'l' => {
                            let is_start = open[col].is_none();
                            if is_start {
                                let lane_u8 = col as u8;
                                register_hold_start(
                                    &mut start_kinds,
                                    time_us,
                                    lane_u8,
                                    col,
                                    step_index,
                                    *line,
                                )?;
                            }

                            toggle_hold(
                                &mut notes,
                                &mut open,
                                resources,
                                col,
                                time_us,
                                step_index,
                                lane_sounds[col].clone(),
                                OpenHoldKind::Charge,
                                *line,
                            )?
                        }
                        'h' => {
                            let is_start = open[col].is_none();
                            if is_start {
                                let lane_u8 = col as u8;
                                register_hold_start(
                                    &mut start_kinds,
                                    time_us,
                                    lane_u8,
                                    col,
                                    step_index,
                                    *line,
                                )?;
                            }

                            toggle_hold(
                                &mut notes,
                                &mut open,
                                resources,
                                col,
                                time_us,
                                step_index,
                                lane_sounds[col].clone(),
                                OpenHoldKind::HellCharge,
                                *line,
                            )?
                        }
                        'b' => {
                            let is_start = open[0].is_none();
                            if is_start {
                                let lane_u8 = 0u8;
                                register_hold_start(
                                    &mut start_kinds,
                                    time_us,
                                    lane_u8,
                                    0,
                                    step_index,
                                    *line,
                                )?;
                            }

                            toggle_scratch_hold_end_se(
                                &mut notes,
                                &mut bgm_events,
                                &mut open,
                                resources,
                                time_us,
                                step_index,
                                sound,
                                lane_sounds[0].clone(),
                                OpenHoldKind::Bss,
                                *line,
                            )?
                        }
                        'B' => {
                            let is_start = open[0].is_none();
                            if is_start {
                                let lane_u8 = 0u8;
                                register_hold_start(
                                    &mut start_kinds,
                                    time_us,
                                    lane_u8,
                                    0,
                                    step_index,
                                    *line,
                                )?;
                            }

                            toggle_scratch_hold_end_se(
                                &mut notes,
                                &mut bgm_events,
                                &mut open,
                                resources,
                                time_us,
                                step_index,
                                sound,
                                lane_sounds[0].clone(),
                                OpenHoldKind::HellBss,
                                *line,
                            )?
                        }
                        'm' => {
                            let is_start = open[0].is_none();
                            if is_start {
                                let lane_u8 = 0u8;
                                register_hold_start(
                                    &mut start_kinds,
                                    time_us,
                                    lane_u8,
                                    0,
                                    step_index,
                                    *line,
                                )?;
                            }

                            toggle_mss(
                                &mut notes,
                                &mut bgm_events,
                                &mut open,
                                resources,
                                time_us,
                                step_index,
                                sound,
                                lane_sounds[0].clone(),
                                OpenHoldKind::Mss { rev: rev.clone() },
                                step_times,
                                *line,
                            )?
                        }
                        'M' => {
                            let is_start = open[0].is_none();
                            if is_start {
                                let lane_u8 = 0u8;
                                register_hold_start(
                                    &mut start_kinds,
                                    time_us,
                                    lane_u8,
                                    0,
                                    step_index,
                                    *line,
                                )?;
                            }

                            toggle_mss(
                                &mut notes,
                                &mut bgm_events,
                                &mut open,
                                resources,
                                time_us,
                                step_index,
                                sound,
                                lane_sounds[0].clone(),
                                OpenHoldKind::HellMss { rev: rev.clone() },
                                step_times,
                                *line,
                            )?
                        }
                        '!' => {
                            handle_marker_checkpoint(
                                &mut open,
                                &mut bgm_events,
                                time_us,
                                step_index,
                                sound,
                                resources,
                                *line,
                            )?;
                        }
                        _ => unreachable!(),
                    }
                }

                step_index += 1;
            }
        }
    }

    // ensure all holds closed
    for (col, v) in open.iter().enumerate() {
        if let Some(h) = v {
            return Err(
                CompileError::new(
                    "E4101",
                    format!(
                        "unclosed toggle (lane={col}, start_line={}, start_time_us={})",
                        h.start_line, h.start_time_us
                    ),
                    h.start_line,
                )
                .with_lane(col as u8)
                .with_step_index(h.start_step_index)
                .with_time_us(h.start_time_us)
                .with_start_line(h.start_line)
                .with_start_time_us(h.start_time_us),
            );
        }
    }

    Ok((notes, bgm_events))
}

fn lane_sounds(sound: &SoundSpec) -> [Option<String>; 8] {
    match sound {
        SoundSpec::None => std::array::from_fn(|_| None),
        SoundSpec::Single(id) => std::array::from_fn(|_| Some(id.clone())),
        SoundSpec::PerLane(lanes) => lanes.clone(),
    }
}

fn validate_sound_id(
    resources: &HashMap<String, String>,
    sound_id: &str,
    line: usize,
    lane: Option<usize>,
) -> Result<(), CompileError> {
    let lane_u8 = lane.and_then(|v| u8::try_from(v).ok());

    if resources.is_empty() {
        let mut err = CompileError::new(
            "E2101",
            match lane {
                Some(lane) => format!(
                    "sound_id referenced but no manifest loaded (sound_id={sound_id}, lane={lane})"
                ),
                None => format!("sound_id referenced but no manifest loaded (sound_id={sound_id})"),
            },
            line,
        )
        .with_sound_id(sound_id);
        if let Some(lane_u8) = lane_u8 {
            err = err.with_lane(lane_u8);
        }
        return Err(err);
    }

    if !resources.contains_key(sound_id) {
        let mut err = CompileError::new(
            "E2101",
            match lane {
                Some(lane) => {
                    format!("sound_id not found in manifest (sound_id={sound_id}, lane={lane})")
                }
                None => format!("sound_id not found in manifest (sound_id={sound_id})"),
            },
            line,
        )
        .with_sound_id(sound_id);
        if let Some(lane_u8) = lane_u8 {
            err = err.with_lane(lane_u8);
        }
        return Err(err);
    }

    Ok(())
}

fn push_bgm_events_from_sound(
    out: &mut Vec<BgmEvent>,
    time_us: Microseconds,
    sound: &SoundSpec,
    resources: &HashMap<String, String>,
    line: usize,
) -> Result<(), CompileError> {
    match sound {
        SoundSpec::None => Ok(()),
        SoundSpec::Single(id) => {
            validate_sound_id(resources, id, line, None)?;
            out.push(BgmEvent {
                time_us,
                sound_id: id.clone(),
            });
            Ok(())
        }
        SoundSpec::PerLane(lanes) => {
            for (lane, id) in lanes.iter().enumerate() {
                let Some(id) = id else { continue };
                validate_sound_id(resources, id, line, Some(lane))?;
                out.push(BgmEvent {
                    time_us,
                    sound_id: id.clone(),
                });
            }
            Ok(())
        }
    }
}

fn toggle_hold(
    notes: &mut Vec<Note>,
    open: &mut [Option<OpenHold>],
    resources: &HashMap<String, String>,
    col: usize,
    time_us: Microseconds,
    step_index: usize,
    sound_id: Option<String>,
    kind: OpenHoldKind,
    line: usize,
) -> Result<(), CompileError> {
    if col == 0 {
        return Err(CompileError::new("E4001", "CN/HCN not allowed on scratch", line));
    }

    match &open[col] {
        None => {
            if let Some(id) = sound_id.as_deref() {
                validate_sound_id(resources, id, line, Some(col))?;
            }
            open[col] = Some(OpenHold {
                start_line: line,
                start_time_us: time_us,
                start_step_index: step_index,
                sound_id,
                kind,
                marker_checkpoints_us: Vec::new(),
            });
        }
        Some(existing) => {
            let (start_time_us, sound_id, existing_kind) =
                (existing.start_time_us, existing.sound_id.clone(), existing.kind.clone());
            match (&existing_kind, &kind) {
                (OpenHoldKind::Charge, OpenHoldKind::Charge)
                | (OpenHoldKind::HellCharge, OpenHoldKind::HellCharge) => {}
                _ => {
                    return Err(CompileError::new(
                        "E4101",
                        "hold type mismatch while toggling",
                        line,
                    ));
                }
            }

            let note_kind = match existing_kind {
                OpenHoldKind::Charge => NoteKind::ChargeNote { end_time_us: time_us },
                OpenHoldKind::HellCharge => NoteKind::HellChargeNote { end_time_us: time_us },
                _ => unreachable!(),
            };

            notes.push(Note {
                time_us: start_time_us,
                col: col as u8,
                kind: note_kind,
                sound_id,
            });
            open[col] = None;
        }
    }
    Ok(())
}

fn toggle_scratch_hold_end_se(
    notes: &mut Vec<Note>,
    bgm_events: &mut Vec<BgmEvent>,
    open: &mut [Option<OpenHold>],
    resources: &HashMap<String, String>,
    time_us: Microseconds,
    step_index: usize,
    end_sound: &SoundSpec,
    start_sound_id: Option<String>,
    kind: OpenHoldKind,
    line: usize,
) -> Result<(), CompileError> {
    if open[0].is_none() {
        if let Some(id) = start_sound_id.as_deref() {
            validate_sound_id(resources, id, line, Some(0))?;
        }
        open[0] = Some(OpenHold {
            start_line: line,
            start_time_us: time_us,
            start_step_index: step_index,
            sound_id: start_sound_id,
            kind,
            marker_checkpoints_us: Vec::new(),
        });
        return Ok(());
    }

    // end
    let existing = open[0].take().unwrap();
    let start_time_us = existing.start_time_us;
    let sound_id = existing.sound_id;
    let existing_kind = existing.kind;

    match (&existing_kind, &kind) {
        (OpenHoldKind::Bss, OpenHoldKind::Bss) | (OpenHoldKind::HellBss, OpenHoldKind::HellBss) => {}
        _ => {
            return Err(CompileError::new(
                "E4101",
                "hold type mismatch while toggling",
                line,
            ));
        }
    }

    // end line SOUND_SPEC -> BgmEvent(s)
    push_bgm_events_from_sound(bgm_events, time_us, end_sound, resources, line)?;

    let note_kind = match existing_kind {
        OpenHoldKind::Bss => NoteKind::BackSpinScratch { end_time_us: time_us },
        OpenHoldKind::HellBss => NoteKind::HellBackSpinScratch { end_time_us: time_us },
        _ => unreachable!(),
    };
    notes.push(Note {
        time_us: start_time_us,
        col: 0,
        kind: note_kind,
        sound_id,
    });

    Ok(())
}

fn toggle_mss(
    notes: &mut Vec<Note>,
    bgm_events: &mut Vec<BgmEvent>,
    open: &mut [Option<OpenHold>],
    resources: &HashMap<String, String>,
    time_us: Microseconds,
    step_index: usize,
    end_sound: &SoundSpec,
    start_sound_id: Option<String>,
    kind: OpenHoldKind,
    step_times: &[Microseconds],
    line: usize,
) -> Result<(), CompileError> {
    if open[0].is_none() {
        // start
        if let Some(id) = start_sound_id.as_deref() {
            validate_sound_id(resources, id, line, Some(0))?;
        }
        open[0] = Some(OpenHold {
            start_line: line,
            start_time_us: time_us,
            start_step_index: step_index,
            sound_id: start_sound_id,
            kind,
            marker_checkpoints_us: Vec::new(),
        });
        return Ok(());
    }

    // end
    let existing = open[0].take().unwrap();
    let start_time_us = existing.start_time_us;
    let sound_id = existing.sound_id;
    let start_step = existing.start_step_index;
    let marker_us = existing.marker_checkpoints_us;
    let existing_kind = existing.kind;

    let (rev, is_hell) = match existing_kind {
        OpenHoldKind::Mss { rev } => (rev, false),
        OpenHoldKind::HellMss { rev } => (rev, true),
        _ => {
            return Err(CompileError::new(
                "E4101",
                "hold type mismatch while toggling",
                line,
            ));
        }
    };

    match (&kind, is_hell) {
        (OpenHoldKind::Mss { .. }, false) | (OpenHoldKind::HellMss { .. }, true) => {}
        _ => {
            return Err(CompileError::new(
                "E4101",
                "hold type mismatch while toggling",
                line,
            ));
        }
    }

    // end line SOUND_SPEC -> BgmEvent(s)
    push_bgm_events_from_sound(bgm_events, time_us, end_sound, resources, line)?;

    let checkpoints = compute_mss_checkpoints(
        start_step,
        step_index,
        time_us,
        step_times,
        &rev,
        &marker_us,
        line,
    )?;
    let note_kind = if is_hell {
        NoteKind::HellMultiSpinScratch {
            end_time_us: time_us,
            reverse_checkpoints_us: checkpoints,
        }
    } else {
        NoteKind::MultiSpinScratch {
            end_time_us: time_us,
            reverse_checkpoints_us: checkpoints,
        }
    };

    notes.push(Note {
        time_us: start_time_us,
        col: 0,
        kind: note_kind,
        sound_id,
    });

    Ok(())
}

fn compute_mss_checkpoints(
    start_step: usize,
    end_step: usize,
    end_time_us: Microseconds,
    step_times: &[Microseconds],
    rev: &RevSpec,
    marker_us: &[Microseconds],
    line: usize,
) -> Result<Vec<Microseconds>, CompileError> {
    if end_step <= start_step {
        return Err(CompileError::new("E4101", "invalid MSS toggle range", line));
    }

    let mut set = HashSet::<Microseconds>::new();
    for &t in marker_us {
        if t != end_time_us {
            set.insert(t);
        }
    }

    if let Some(n) = rev.every {
        let mut idx = start_step + n;
        while idx < end_step {
            if let Some(&t) = step_times.get(idx) {
                if t != end_time_us {
                    set.insert(t);
                }
            }
            idx += n;
        }
    }

    for &a in &rev.at {
        // a is 1-based step number from start, and must be >= 2
        let idx = start_step + (a - 1);
        if idx < end_step {
            if let Some(&t) = step_times.get(idx) {
                if t != end_time_us {
                    set.insert(t);
                }
            }
        }
    }

    let mut v: Vec<Microseconds> = set.into_iter().collect();
    v.sort_unstable();
    Ok(v)
}

pub(crate) fn compute_total_duration_us(notes: &[Note], bgm_events: &[BgmEvent]) -> Microseconds {
    let mut max_us: Microseconds = 0;
    for n in notes {
        let end = match &n.kind {
            NoteKind::Tap => n.time_us,
            NoteKind::ChargeNote { end_time_us }
            | NoteKind::HellChargeNote { end_time_us }
            | NoteKind::BackSpinScratch { end_time_us }
            | NoteKind::HellBackSpinScratch { end_time_us }
            | NoteKind::MultiSpinScratch { end_time_us, .. }
            | NoteKind::HellMultiSpinScratch { end_time_us, .. } => (*end_time_us).max(n.time_us),
        };
        max_us = max_us.max(end);
    }
    for e in bgm_events {
        max_us = max_us.max(e.time_us);
    }
    max_us
}
