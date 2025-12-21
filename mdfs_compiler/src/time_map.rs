use mdf_schema::Microseconds;

use crate::CompileError;
use crate::parser::{Directive, TrackLine};

pub(crate) fn pass1_time_map(
    track: &[TrackLine],
) -> Result<(Vec<Microseconds>, Vec<Microseconds>), CompileError> {
    let mut bpm: Option<f64> = None;
    let mut div: Option<u32> = None;
    let mut current_time_us: Microseconds = 0;
    let mut starts = Vec::new();
    let mut durs = Vec::new();

    for line in track {
        match line {
            TrackLine::Directive { line: _line, directive } => match directive {
                Directive::Bpm(v) => bpm = Some(*v),
                Directive::Div(v) => div = Some(*v),
            },
            TrackLine::Step { line, .. } => {
                let bpm = bpm
                    .ok_or_else(|| CompileError::new("E3001", "@bpm is required before step lines", *line))?;
                let div = div
                    .ok_or_else(|| CompileError::new("E3002", "@div is required before step lines", *line))?;
                let dur = step_duration_us(bpm, div, *line)?;
                starts.push(current_time_us);
                durs.push(dur);
                current_time_us = current_time_us
                    .checked_add(dur)
                    .ok_or_else(|| CompileError::new("E3005", "time overflow", *line))?;
            }
        }
    }
    Ok((starts, durs))
}

fn step_duration_us(bpm: f64, div: u32, line: usize) -> Result<Microseconds, CompileError> {
    if !(bpm > 0.0) {
        return Err(CompileError::new("E3003", "@bpm must be > 0", line));
    }
    if div < 1 {
        return Err(CompileError::new("E3004", "@div must be >= 1", line));
    }
    let step_duration_sec = (60.0 / bpm) * (4.0 / div as f64);
    let us_f64 = step_duration_sec * 1_000_000.0;
    let us = (us_f64 + 0.5).floor() as Microseconds;
    if us == 0 {
        return Err(CompileError::new(
            "E3005",
            "step duration rounded to 0us; bpm/div too extreme",
            line,
        ));
    }
    Ok(us)
}
