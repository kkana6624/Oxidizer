use crate::CompileError;

#[derive(Debug, Default, Clone)]
pub(crate) struct ParsedMeta {
    pub(crate) title: Option<String>,
    pub(crate) artist: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) sound_manifest: Option<String>,
    pub(crate) sound_manifest_line: Option<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedMdfs {
    pub(crate) meta: ParsedMeta,
    pub(crate) meta_line: usize,
    pub(crate) track: Vec<TrackLine>,
}

#[derive(Debug, Clone)]
pub(crate) enum TrackLine {
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
pub(crate) enum Directive {
    Bpm(f64),
    Div(u32),
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RevSpec {
    pub(crate) every: Option<usize>,
    pub(crate) at: Vec<usize>,
}

#[derive(Debug, Clone)]
pub(crate) enum SoundSpec {
    None,
    Single(String),
    PerLane([Option<String>; 8]),
}

pub(crate) fn parse_mdfs(src: &str) -> Result<ParsedMdfs, CompileError> {
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
                    format!(
                        "metadata directive not allowed inside track body: @{directive_name}"
                    ),
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

fn parse_header_directive(
    meta: &mut ParsedMeta,
    trimmed: &str,
    line_no: usize,
) -> Result<(), CompileError> {
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
            return Err(
                CompileError::new(
                    "E4001",
                    format!(
                        "undefined step char (lane={idx}, char='{ch}', context={trimmed})"
                    ),
                    line_no,
                )
                .with_lane(idx as u8)
                .with_context(trimmed.to_string()),
            );
        }

        if idx != 0 && matches!(ch, 'S' | 'b' | 'm' | 'B' | 'M') {
            return Err(
                CompileError::new(
                    "E4002",
                    format!(
                        "scratch-only char used on non-scratch lane (lane={idx}, char='{ch}', context={trimmed})"
                    ),
                    line_no,
                )
                .with_lane(idx as u8)
                .with_context(trimmed.to_string()),
            );
        }

        if idx != 0 && ch == '!' {
            return Err(
                CompileError::new(
                    "E4003",
                    format!(
                        "'!' is only allowed on scratch lane (lane=0) (lane={idx}, context={trimmed})"
                    ),
                    line_no,
                )
                .with_lane(idx as u8)
                .with_context(trimmed.to_string()),
            );
        }

        if idx == 0 && matches!(ch, 'l' | 'h') {
            return Err(
                CompileError::new(
                    "E4001",
                    format!(
                        "char not allowed on scratch lane (lane=0, char='{ch}', context={trimmed})"
                    ),
                    line_no,
                )
                .with_lane(0)
                .with_context(trimmed.to_string()),
            );
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

fn parse_step_tail(
    tail: &str,
    context_line: &str,
    line_no: usize,
) -> Result<(SoundSpec, RevSpec), CompileError> {
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
                return Err(
                    CompileError::new(
                        "E1005",
                        format!("@rev_every must be >= 1 (context={context_line})"),
                        line_no,
                    )
                    .with_context(context_line.to_string()),
                );
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
                return Err(
                    CompileError::new(
                        "E1004",
                        format!("empty @rev_at list (context={context_line})"),
                        line_no,
                    )
                    .with_context(context_line.to_string()),
                );
            }
            let mut values = Vec::new();
            for part in list.split(',') {
                let p = part.trim();
                if p.is_empty() {
                    return Err(
                        CompileError::new(
                            "E1004",
                            format!("invalid @rev_at list (context={context_line})"),
                            line_no,
                        )
                        .with_context(context_line.to_string()),
                    );
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
                    return Err(
                        CompileError::new(
                            "E1004",
                            format!("@rev_at values must be >= 2 (context={context_line})"),
                            line_no,
                        )
                        .with_context(context_line.to_string()),
                    );
                }
                values.push(v);
            }
            spec.at = values;
            rest = next.trim_start();
            continue;
        }

        return Err(
            CompileError::new(
                "E1006",
                format!("unexpected trailing tokens: {rest} (context={context_line})"),
                line_no,
            )
            .with_context(context_line.to_string()),
        );
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
            return Err(
                CompileError::new(
                    "E1001",
                    format!("invalid SOUND_SPEC array (context={context_line})"),
                    line_no,
                )
                .with_context(context_line.to_string()),
            );
        }
        let inner = &s[1..s.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() != 8 {
            return Err(
                CompileError::new(
                    "E1002",
                    format!("SOUND_SPEC lane array must have 8 slots (context={context_line})"),
                    line_no,
                )
                .with_context(context_line.to_string()),
            );
        }
        let mut lanes: [Option<String>; 8] = std::array::from_fn(|_| None);
        for (i, p) in parts.iter().enumerate() {
            if p.is_empty() {
                return Err(
                    CompileError::new(
                        "E1003",
                        format!("invalid SOUND_SPEC slot (lane={i}, context={context_line})"),
                        line_no,
                    )
                    .with_lane(i as u8)
                    .with_context(context_line.to_string()),
                );
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
        return Err(
            CompileError::new(
                "E1001",
                format!("invalid SOUND_SPEC token (context={context_line})"),
                line_no,
            )
            .with_context(context_line.to_string()),
        );
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
