use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use mdf_schema::{BgmEvent, Metadata, Microseconds, MdfChart, Note, NoteKind, SpeedEvent, VisualEvent};
use thiserror::Error;

#[derive(Debug, Clone, Default)]
pub struct CompileOptions {
    pub base_dir: Option<PathBuf>,
}

#[derive(Debug, Error, Clone)]
#[error("{code}: {message} (line {line})")]
pub struct CompileError {
    pub code: &'static str,
    pub kind: CompileErrorKind,
    pub message: String,
    pub line: usize,

    // --- Structured fields (MVP: optional, message stays source-of-truth) ---
    pub file: Option<String>,
    pub column: Option<usize>,
    pub step_index: Option<usize>,
    pub lane: Option<u8>,
    pub time_us: Option<Microseconds>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileErrorKind {
    Parse,
    Semantic,
    IO,
    TimeMap,
    Validation,
}

impl CompileErrorKind {
    fn from_code(code: &'static str) -> Self {
        // Spec: docs/MDFS_DSL-and-Compiler_Spec.md#6.2
        match code {
            // Parse
            "E1001" | "E1002" | "E1003" | "E1004" | "E1005" | "E1006" | "E1101" | "E3201" | "E3202"
            | "E3203" | "E3204" => Self::Parse,

            // IO
            "E2001" | "E2002" | "E2003" | "E2004" => Self::IO,

            // Semantic
            "E2101" | "E4201" => Self::Semantic,

            // TimeMap
            "E3001" | "E3002" | "E3003" | "E3004" | "E3005" => Self::TimeMap,

            // Validation
            "E4001" | "E4002" | "E4003" | "E4004" | "E4101" | "E4102" => Self::Validation,

            // MVP default: treat unknown codes as Parse.
            _ => Self::Parse,
        }
    }
}

impl CompileError {
    fn new(code: &'static str, message: impl Into<String>, line: usize) -> Self {
        Self {
            code,
            kind: CompileErrorKind::from_code(code),
            message: message.into(),
            line,

            file: None,
            column: None,
            step_index: None,
            lane: None,
            time_us: None,
            context: None,
        }
    }

    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    pub fn with_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    pub fn with_step_index(mut self, step_index: usize) -> Self {
        self.step_index = Some(step_index);
        self
    }

    pub fn with_lane(mut self, lane: u8) -> Self {
        self.lane = Some(lane);
        self
    }

    pub fn with_time_us(mut self, time_us: Microseconds) -> Self {
        self.time_us = Some(time_us);
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

pub fn compile_file(path: impl AsRef<Path>) -> Result<MdfChart, CompileError> {
    let path = path.as_ref();
    let src = fs::read_to_string(path)
           .map_err(|e| {
               CompileError::new("E2001", format!("failed to read input .mdfs: {e}"), 0)
                   .with_file(path.display().to_string())
           })?;
    let base_dir = path.parent().map(|p| p.to_path_buf());
    compile_str_with_options(&src, CompileOptions { base_dir })
}

pub fn compile_str(src: &str) -> Result<MdfChart, CompileError> {
    compile_str_with_options(src, CompileOptions::default())
}

pub fn compile_str_with_options(src: &str, options: CompileOptions) -> Result<MdfChart, CompileError> {
    let parsed = parse_mdfs(src)?;

    let resources = load_resources(&parsed, &options)?;
    let (step_times, _step_durations) = pass1_time_map(&parsed.track)?;
    let (mut notes, mut bgm_events) = pass2_generate(&parsed.track, &step_times, &resources)?;

    notes.sort_by_key(|n| n.time_us);
    bgm_events.sort_by_key(|e| e.time_us);

    let total_duration_us = compute_total_duration_us(&notes, &bgm_events);
    let meta = Metadata {
        title: parsed.meta.title.ok_or_else(|| {
            CompileError::new("E3201", "missing @title", parsed.meta_line)
        })?,
        artist: parsed.meta.artist.ok_or_else(|| {
            CompileError::new("E3202", "missing @artist", parsed.meta_line)
        })?,
        version: parsed.meta.version.ok_or_else(|| {
            CompileError::new("E3203", "missing @version", parsed.meta_line)
        })?,
        tags: parsed.meta.tags,
        total_duration_us,
    };

    Ok(MdfChart {
        meta,
        resources,
        visual_events: Vec::<VisualEvent>::new(),
        speed_events: Vec::<SpeedEvent>::new(),
        notes,
        bgm_events,
    })
}

#[derive(Debug, Default, Clone)]
struct ParsedMeta {
    title: Option<String>,
    artist: Option<String>,
    version: Option<String>,
    tags: Vec<String>,
    sound_manifest: Option<String>,
    sound_manifest_line: Option<usize>,
}

#[derive(Debug, Clone)]
struct ParsedMdfs {
    meta: ParsedMeta,
    meta_line: usize,
    track: Vec<TrackLine>,
}

#[derive(Debug, Clone)]
enum TrackLine {
    Directive {
        line: usize,
        directive: Directive,
    },
    Step {
        line: usize,
        cells: [char; 8],
        sound: SoundSpec,
        rev: RevSpec,
    },
}

#[derive(Debug, Clone)]
enum Directive {
    Bpm(f64),
    Div(u32),
}

#[derive(Debug, Clone, Default)]
struct RevSpec {
    every: Option<usize>,
    at: Vec<usize>,
}

#[derive(Debug, Clone)]
enum SoundSpec {
    None,
    Single(String),
    PerLane([Option<String>; 8]),
}

fn parse_mdfs(src: &str) -> Result<ParsedMdfs, CompileError> {
    let mut meta = ParsedMeta::default();
    let mut track = Vec::new();
    let mut in_track = false;
    let mut meta_line = 1;

    for (i, raw_line) in src.lines().enumerate() {
        let line_no = i + 1;
        let line = strip_inline_comment(raw_line);
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }

        if !in_track {
            if trimmed == "track: |" {
                in_track = true;
                meta_line = line_no;
                continue;
            }

            if trimmed.starts_with('@') {
                parse_header_directive(&mut meta, trimmed, line_no)?;
                continue;
            }

                return Err(CompileError::new(
                    "E1101",
                    "unexpected content before track: |",
                    line_no,
                ));
        }

        // track body
        if trimmed.starts_with('@') {
            // MVP: header-like directives inside body are errors (avoid ambiguity)
            let directive_name = trimmed
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_start_matches('@');
            if matches!(
                directive_name,
                "title" | "artist" | "version" | "tags" | "sound_manifest"
            ) {
                return Err(CompileError::new(
                        "E1006",
                    format!("metadata directive not allowed inside track body: @{directive_name}"),
                    line_no,
                ));
            }
            if let Some(d) = parse_track_directive(trimmed, line_no)? {
                track.push(TrackLine::Directive {
                    line: line_no,
                    directive: d,
                });
                continue;
            }

            return Err(CompileError::new(
                    "E1006",
                format!("unknown directive: {trimmed}"),
                line_no,
            ));
        }

        let step = parse_step_line(trimmed, line_no)?;
        track.push(step);
    }

    if !in_track {
        return Err(CompileError::new("E1101", "missing track: |", 0));
    }

    Ok(ParsedMdfs {
        meta,
        meta_line,
        track,
    })
}

fn parse_header_directive(meta: &mut ParsedMeta, trimmed: &str, line_no: usize) -> Result<(), CompileError> {
    let (name, rest) = split_directive(trimmed, line_no)?;
    match name {
        "title" => meta.title = Some(rest.to_string()),
        "artist" => meta.artist = Some(rest.to_string()),
        "version" => meta.version = Some(rest.to_string()),
        "tags" => meta.tags = parse_tags_csv(rest, line_no)?,
        "sound_manifest" => {
            if meta.sound_manifest.is_some() {
                return Err(CompileError::new(
                    "E2004",
                    "@sound_manifest specified multiple times",
                    line_no,
                ));
            }
            if rest.is_empty() {
                return Err(CompileError::new("E2001", "missing manifest path", line_no));
            }
            meta.sound_manifest = Some(rest.to_string());
            meta.sound_manifest_line = Some(line_no);
        }
        _ => {
            return Err(CompileError::new(
                "E1006",
                format!("unknown header directive: @{name}"),
                line_no,
            ));
        }
    }
    Ok(())
}

fn parse_track_directive(trimmed: &str, line_no: usize) -> Result<Option<Directive>, CompileError> {
    let (name, rest) = split_directive(trimmed, line_no)?;
    match name {
        "bpm" => {
            let bpm: f64 = rest
                .parse()
                .map_err(|_| CompileError::new("E3003", "invalid @bpm", line_no))?;
            if !(bpm > 0.0) {
                return Err(CompileError::new("E3003", "@bpm must be > 0", line_no));
            }
            Ok(Some(Directive::Bpm(bpm)))
        }
        "div" => {
            let div: i64 = rest
                .parse()
                .map_err(|_| CompileError::new("E3004", "invalid @div", line_no))?;
            if div < 1 {
                return Err(CompileError::new("E3004", "@div must be >= 1", line_no));
            }
            Ok(Some(Directive::Div(div as u32)))
        }
        _ => Ok(None),
    }
}

fn parse_step_line(trimmed: &str, line_no: usize) -> Result<TrackLine, CompileError> {
    let mut chars = trimmed.chars();
    let mut cells = ['.'; 8];
    for idx in 0..8 {
        cells[idx] = chars
            .next()
            .ok_or_else(|| {
                CompileError::new(
                    "E1101",
                    format!("step line must have 8 chars (context={trimmed})"),
                    line_no,
                )
                .with_context(trimmed.to_string())
            })?;
    }

    for (idx, &ch) in cells.iter().enumerate() {
        let ok = matches!(ch, '.' | 'N' | 'S' | 'l' | 'h' | 'b' | 'm' | 'B' | 'M' | '!');
        if !ok {
            return Err(CompileError::new(
                "E4001",
                format!("undefined step char (lane={idx}, char='{ch}', context={trimmed})"),
                line_no,
            )
            .with_lane(idx as u8)
            .with_context(trimmed.to_string()));
        }

        if idx != 0 && matches!(ch, 'S' | 'b' | 'm' | 'B' | 'M') {
            return Err(CompileError::new(
                "E4002",
                format!("scratch-only char used on non-scratch lane (lane={idx}, char='{ch}', context={trimmed})"),
                line_no,
            )
            .with_lane(idx as u8)
            .with_context(trimmed.to_string()));
        }

        if idx != 0 && ch == '!' {
            return Err(CompileError::new(
                "E4003",
                format!("'!' is only allowed on scratch lane (lane=0) (lane={idx}, context={trimmed})"),
                line_no,
            )
            .with_lane(idx as u8)
            .with_context(trimmed.to_string()));
        }

        if idx == 0 && matches!(ch, 'l' | 'h') {
            return Err(CompileError::new(
                "E4001",
                format!("char not allowed on scratch lane (lane=0, char='{ch}', context={trimmed})"),
                line_no,
            )
            .with_lane(0)
            .with_context(trimmed.to_string()));
        }
    }

    let tail = chars.as_str().trim();
    let (sound, rev) = parse_step_tail(tail, trimmed, line_no)?;

    Ok(TrackLine::Step {
        line: line_no,
        cells,
        sound,
        rev,
    })
}

fn parse_step_tail(tail: &str, context_line: &str, line_no: usize) -> Result<(SoundSpec, RevSpec), CompileError> {
    if tail.is_empty() {
        return Ok((SoundSpec::None, RevSpec::default()));
    }

    let mut sound = SoundSpec::None;
    let mut rev = RevSpec::default();

    let mut rest = tail.trim();
    if let Some(colon_idx) = rest.find(':') {
        let after = rest[(colon_idx + 1)..].trim();
        // split sound and rev directives (if any)
        let (sound_part, rev_part) = split_sound_and_rev(after);
        sound = parse_sound_spec(sound_part.trim(), context_line, line_no)?;
        rest = rev_part.trim();
    }

    if !rest.is_empty() {
        rev = parse_rev_spec(rest, context_line, line_no)?;
    }

    Ok((sound, rev))
}

fn split_sound_and_rev(after_colon: &str) -> (&str, &str) {
    let rev_every = after_colon.find("@rev_every");
    let rev_at = after_colon.find("@rev_at");
    let idx = match (rev_every, rev_at) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };

    match idx {
        Some(i) => (&after_colon[..i], &after_colon[i..]),
        None => (after_colon, ""),
    }
}

fn parse_rev_spec(s: &str, context_line: &str, line_no: usize) -> Result<RevSpec, CompileError> {
    let mut spec = RevSpec::default();
    let mut rest = s.trim();

    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("@rev_every") {
            rest = after.trim_start();
            let (tok, next) = split_first_token(rest);
            let n: usize = tok
                .parse()
                .map_err(|_| {
                    CompileError::new(
                        "E1005",
                        format!("invalid @rev_every (context={context_line})"),
                        line_no,
                    )
                    .with_context(context_line.to_string())
                })?;
            if n < 1 {
                return Err(CompileError::new(
                    "E1005",
                    format!("@rev_every must be >= 1 (context={context_line})"),
                    line_no,
                )
                .with_context(context_line.to_string()));
            }
            spec.every = Some(n);
            rest = next.trim_start();
            continue;
        }

        if let Some(after) = rest.strip_prefix("@rev_at") {
            rest = after.trim_start();
            let (tok, next) = split_first_token(rest);
            let list = tok.trim();
            if list.is_empty() {
                return Err(CompileError::new(
                    "E1004",
                    format!("empty @rev_at list (context={context_line})"),
                    line_no,
                )
                .with_context(context_line.to_string()));
            }
            let mut values = Vec::new();
            for part in list.split(',') {
                let p = part.trim();
                if p.is_empty() {
                    return Err(CompileError::new(
                        "E1004",
                        format!("invalid @rev_at list (context={context_line})"),
                        line_no,
                    )
                    .with_context(context_line.to_string()));
                }
                let v: usize = p
                    .parse()
                    .map_err(|_| {
                        CompileError::new(
                            "E1004",
                            format!("invalid @rev_at list (context={context_line})"),
                            line_no,
                        )
                        .with_context(context_line.to_string())
                    })?;
                if v < 2 {
                    return Err(CompileError::new(
                        "E1004",
                        format!("@rev_at values must be >= 2 (context={context_line})"),
                        line_no,
                    )
                    .with_context(context_line.to_string()));
                }
                values.push(v);
            }
            spec.at = values;
            rest = next.trim_start();
            continue;
        }

        return Err(CompileError::new(
            "E1006",
            format!("unexpected trailing tokens: {rest} (context={context_line})"),
            line_no,
        )
        .with_context(context_line.to_string()));
    }

    Ok(spec)
}

fn split_first_token(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    match s.find(char::is_whitespace) {
        Some(i) => (&s[..i], &s[i..]),
        None => (s, ""),
    }
}

fn parse_sound_spec(s: &str, context_line: &str, line_no: usize) -> Result<SoundSpec, CompileError> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(SoundSpec::None);
    }

    if s == "[]" {
        return Ok(SoundSpec::None);
    }

    if s.starts_with('[') {
        if !s.ends_with(']') {
            return Err(CompileError::new(
                "E1001",
                format!("invalid SOUND_SPEC array (context={context_line})"),
                line_no,
            )
            .with_context(context_line.to_string()));
        }
        let inner = &s[1..s.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() != 8 {
            return Err(CompileError::new(
                "E1002",
                format!("SOUND_SPEC lane array must have 8 slots (context={context_line})"),
                line_no,
            )
            .with_context(context_line.to_string()));
        }
        let mut lanes: [Option<String>; 8] = std::array::from_fn(|_| None);
        for (i, p) in parts.iter().enumerate() {
            if p.is_empty() {
                return Err(CompileError::new(
                    "E1003",
                    format!("invalid SOUND_SPEC slot (lane={i}, context={context_line})"),
                    line_no,
                )
                .with_lane(i as u8)
                .with_context(context_line.to_string()));
            }
            if *p == "-" {
                lanes[i] = None;
            } else {
                lanes[i] = Some((*p).to_string());
            }
        }
        return Ok(SoundSpec::PerLane(lanes));
    }

    if s.contains(char::is_whitespace) {
        return Err(CompileError::new(
            "E1001",
            format!("invalid SOUND_SPEC token (context={context_line})"),
            line_no,
        )
        .with_context(context_line.to_string()));
    }
    Ok(SoundSpec::Single(s.to_string()))
}

fn parse_tags_csv(s: &str, line_no: usize) -> Result<Vec<String>, CompileError> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(vec![]);
    }
    let mut tags = Vec::new();
    for part in s.split(',') {
        let t = part.trim();
        if t.is_empty() {
            return Err(CompileError::new(
                "E3204",
                format!("invalid @tags csv (context=@tags {s})"),
                line_no,
            ));
        }
        tags.push(t.to_string());
    }
    Ok(tags)
}

fn split_directive(trimmed: &str, line_no: usize) -> Result<(&str, &str), CompileError> {
    let mut iter = trimmed.splitn(2, char::is_whitespace);
    let head = iter.next().unwrap_or("");
    if !head.starts_with('@') {
        return Err(CompileError::new("E1006", "expected directive", line_no));
    }
    let name = head.trim_start_matches('@');
    let rest = iter.next().unwrap_or("").trim();
    Ok((name, rest))
}

fn strip_inline_comment(line: &str) -> &str {
    match line.find('#') {
        Some(i) => &line[..i],
        None => line,
    }
}

fn load_resources(parsed: &ParsedMdfs, options: &CompileOptions) -> Result<HashMap<String, String>, CompileError> {
    let Some(manifest_path) = &parsed.meta.sound_manifest else {
        return Ok(HashMap::new());
    };

    let manifest_line = parsed.meta.sound_manifest_line.unwrap_or(parsed.meta_line);

    let Some(base_dir) = &options.base_dir else {
        return Err(CompileError::new(
            "E2001",
            "@sound_manifest requires compile_file() or CompileOptions.base_dir",
            manifest_line,
        ));
    };

    let full = base_dir.join(manifest_path);
    let bytes = fs::read(&full).map_err(|e| {
        CompileError::new(
            "E2001",
            format!("failed to read manifest {}: {e}", full.display()),
            manifest_line,
        )
    })?;

    let map: HashMap<String, serde_json::Value> = serde_json::from_slice(&bytes)
        .map_err(|e| CompileError::new("E2002", format!("invalid manifest json: {e}"), manifest_line))?;

    let mut out = HashMap::new();
    for (k, v) in map {
        let Some(s) = v.as_str() else {
            return Err(CompileError::new(
                "E2003",
                "manifest values must be strings",
                manifest_line,
            ));
        };
        if k.trim().is_empty() || s.trim().is_empty() {
            return Err(CompileError::new(
                "E2003",
                "manifest keys/values must be non-empty",
                manifest_line,
            ));
        }
        out.insert(k, s.to_string());
    }
    Ok(out)
}

fn pass1_time_map(track: &[TrackLine]) -> Result<(Vec<Microseconds>, Vec<Microseconds>), CompileError> {
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
                let bpm = bpm.ok_or_else(|| CompileError::new("E3001", "@bpm is required before step lines", *line))?;
                let div = div.ok_or_else(|| CompileError::new("E3002", "@div is required before step lines", *line))?;
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

fn pass2_generate(
    track: &[TrackLine],
    step_times: &[Microseconds],
    resources: &HashMap<String, String>,
) -> Result<(Vec<Note>, Vec<BgmEvent>), CompileError> {
    let mut notes = Vec::new();
    let mut bgm_events = Vec::new();

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum StartKind {
        Tap,
        HoldStart,
    }
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
                    return Err(CompileError::new(
                        "E4201",
                        "@rev_every/@rev_at only allowed on MSS/HMSS start line",
                        *line,
                    )
                    .with_step_index(step_index)
                    .with_time_us(time_us));
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
                            if let Some(existing) = start_kinds.get(&(time_us, lane_u8)) {
                                if *existing == StartKind::HoldStart {
                                    return Err(CompileError::new(
                                        "E4004",
                                        format!(
                                            "tap overlaps hold start at same (time_us,lane) (time_us={time_us}, lane={col})"
                                        ),
                                        *line,
                                    )
                                    .with_step_index(step_index)
                                    .with_time_us(time_us)
                                    .with_lane(lane_u8));
                                }
                            } else {
                                start_kinds.insert((time_us, lane_u8), StartKind::Tap);
                            }

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
                                if let Some(existing) = start_kinds.get(&(time_us, lane_u8)) {
                                    if *existing == StartKind::Tap {
                                        return Err(CompileError::new(
                                            "E4004",
                                            format!(
                                                "hold start overlaps tap at same (time_us,lane) (time_us={time_us}, lane={col})"
                                            ),
                                            *line,
                                        )
                                        .with_step_index(step_index)
                                        .with_time_us(time_us)
                                        .with_lane(lane_u8));
                                    }
                                } else {
                                    start_kinds.insert((time_us, lane_u8), StartKind::HoldStart);
                                }
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
                                if let Some(existing) = start_kinds.get(&(time_us, lane_u8)) {
                                    if *existing == StartKind::Tap {
                                        return Err(CompileError::new(
                                            "E4004",
                                            format!(
                                                "hold start overlaps tap at same (time_us,lane) (time_us={time_us}, lane={col})"
                                            ),
                                            *line,
                                        )
                                        .with_step_index(step_index)
                                        .with_time_us(time_us)
                                        .with_lane(lane_u8));
                                    }
                                } else {
                                    start_kinds.insert((time_us, lane_u8), StartKind::HoldStart);
                                }
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
                                if let Some(existing) = start_kinds.get(&(time_us, lane_u8)) {
                                    if *existing == StartKind::Tap {
                                        return Err(CompileError::new(
                                            "E4004",
                                            format!(
                                                "hold start overlaps tap at same (time_us,lane) (time_us={time_us}, lane=0)"
                                            ),
                                            *line,
                                        )
                                        .with_step_index(step_index)
                                        .with_time_us(time_us)
                                        .with_lane(0));
                                    }
                                } else {
                                    start_kinds.insert((time_us, lane_u8), StartKind::HoldStart);
                                }
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
                                if let Some(existing) = start_kinds.get(&(time_us, lane_u8)) {
                                    if *existing == StartKind::Tap {
                                        return Err(CompileError::new(
                                            "E4004",
                                            format!(
                                                "hold start overlaps tap at same (time_us,lane) (time_us={time_us}, lane=0)"
                                            ),
                                            *line,
                                        )
                                        .with_step_index(step_index)
                                        .with_time_us(time_us)
                                        .with_lane(0));
                                    }
                                } else {
                                    start_kinds.insert((time_us, lane_u8), StartKind::HoldStart);
                                }
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
                                if let Some(existing) = start_kinds.get(&(time_us, lane_u8)) {
                                    if *existing == StartKind::Tap {
                                        return Err(CompileError::new(
                                            "E4004",
                                            format!(
                                                "hold start overlaps tap at same (time_us,lane) (time_us={time_us}, lane=0)"
                                            ),
                                            *line,
                                        )
                                        .with_step_index(step_index)
                                        .with_time_us(time_us)
                                        .with_lane(0));
                                    }
                                } else {
                                    start_kinds.insert((time_us, lane_u8), StartKind::HoldStart);
                                }
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
                                if let Some(existing) = start_kinds.get(&(time_us, lane_u8)) {
                                    if *existing == StartKind::Tap {
                                        return Err(CompileError::new(
                                            "E4004",
                                            format!(
                                                "hold start overlaps tap at same (time_us,lane) (time_us={time_us}, lane=0)"
                                            ),
                                            *line,
                                        )
                                        .with_step_index(step_index)
                                        .with_time_us(time_us)
                                        .with_lane(0));
                                    }
                                } else {
                                    start_kinds.insert((time_us, lane_u8), StartKind::HoldStart);
                                }
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
                            // marker checkpoint only valid inside MSS/HMSS hold
                            let Some(open0) = &mut open[0] else {
                                return Err(CompileError::new(
                                    "E4003",
                                    "'!' is only valid while MSS/HMSS is active",
                                    *line,
                                )
                                .with_step_index(step_index)
                                .with_time_us(time_us)
                                .with_lane(0));
                            };

                            match open0.kind {
                                OpenHoldKind::Mss { .. } | OpenHoldKind::HellMss { .. } => {
                                    open0.marker_checkpoints_us.push(time_us);
                                    push_bgm_events_from_sound(&mut bgm_events, time_us, sound, resources, *line)?;
                                }
                                OpenHoldKind::Bss | OpenHoldKind::HellBss => {
                                    return Err(CompileError::new(
                                        "E4102",
                                        "'!' is not allowed while BSS/HBSS is active",
                                        *line,
                                    )
                                    .with_step_index(step_index)
                                    .with_time_us(time_us)
                                    .with_lane(0));
                                }
                                _ => {
                                    return Err(CompileError::new(
                                        "E4003",
                                        "'!' is only valid while MSS/HMSS is active",
                                        *line,
                                    )
                                    .with_step_index(step_index)
                                    .with_time_us(time_us)
                                    .with_lane(0));
                                }
                            }
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
            return Err(CompileError::new(
                "E4101",
                format!(
                    "unclosed toggle (lane={col}, start_line={}, start_time_us={})",
                    h.start_line,
                    h.start_time_us
                ),
                h.start_line,
            )
            .with_lane(col as u8)
            .with_step_index(h.start_step_index)
            .with_time_us(h.start_time_us));
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
                Some(lane) => {
                    format!(
                        "sound_id referenced but no manifest loaded (sound_id={sound_id}, lane={lane})"
                    )
                }
                None => format!(
                    "sound_id referenced but no manifest loaded (sound_id={sound_id})"
                ),
            },
            line,
        );
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
        );
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
                OpenHoldKind::Charge => NoteKind::ChargeNote {
                    end_time_us: time_us,
                },
                OpenHoldKind::HellCharge => NoteKind::HellChargeNote {
                    end_time_us: time_us,
                },
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
        OpenHoldKind::Bss => NoteKind::BackSpinScratch {
            end_time_us: time_us,
        },
        OpenHoldKind::HellBss => NoteKind::HellBackSpinScratch {
            end_time_us: time_us,
        },
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

    let checkpoints = compute_mss_checkpoints(start_step, step_index, time_us, step_times, &rev, &marker_us, line)?;
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


fn compute_total_duration_us(notes: &[Note], bgm_events: &[BgmEvent]) -> Microseconds {
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

#[cfg(test)]
mod tests;
