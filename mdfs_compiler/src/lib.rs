use std::{
    fs,
    path::{Path, PathBuf},
};

use mdf_schema::{Metadata, MdfChart, SpeedEvent, VisualEvent};

mod error;
mod generate;
mod parser;
mod resources;
mod time_map;

pub use error::{CompileError, CompileErrorKind};

/// Options for compilation.
///
/// MVP: currently only controls how relative paths (e.g. `@sound_manifest`) are resolved.
#[derive(Debug, Clone, Default)]
pub struct CompileOptions {
    /// Base directory used to resolve relative paths.
    ///
    /// - `compile_file()` sets this automatically to the input file's parent directory.
    /// - `compile_str()` uses `None` by default.
    pub base_dir: Option<PathBuf>,
}

/// Compile an `.mdfs` file into an `MdfChart`.
///
/// Returns `CompileError` on failure. Its `Display` output is stable and only includes
/// `code`, `message` and `line` (structured fields are available separately).
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

/// Compile `.mdfs` source text into an `MdfChart`.
pub fn compile_str(src: &str) -> Result<MdfChart, CompileError> {
    compile_str_with_options(src, CompileOptions::default())
}

/// Compile `.mdfs` source text into an `MdfChart` with options.
pub fn compile_str_with_options(src: &str, options: CompileOptions) -> Result<MdfChart, CompileError> {
    let parsed = parser::parse_mdfs(src)?;

    let resources = resources::load_resources(&parsed, &options)?;
    let (step_times, _step_durations) = time_map::pass1_time_map(&parsed.track)?;
    let (mut notes, mut bgm_events) = generate::pass2_generate(&parsed.track, &step_times, &resources)?;

    notes.sort_by_key(|n| n.time_us);
    bgm_events.sort_by_key(|e| e.time_us);

    let total_duration_us = generate::compute_total_duration_us(&notes, &bgm_events);
    let meta = Metadata {
        title: parsed
            .meta
            .title
            .ok_or_else(|| CompileError::new("E3201", "missing @title", parsed.meta_line))?,
        artist: parsed
            .meta
            .artist
            .ok_or_else(|| CompileError::new("E3202", "missing @artist", parsed.meta_line))?,
        version: parsed
            .meta
            .version
            .ok_or_else(|| CompileError::new("E3203", "missing @version", parsed.meta_line))?,
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

#[cfg(test)]
mod tests;
