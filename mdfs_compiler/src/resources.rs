use std::{collections::HashMap, fs};

use crate::{CompileError, CompileOptions};
use crate::parser::ParsedMdfs;

pub(crate) fn load_resources(
    parsed: &ParsedMdfs,
    options: &CompileOptions,
) -> Result<HashMap<String, String>, CompileError> {
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
        .with_file(full.display().to_string())
    })?;

    let map: HashMap<String, serde_json::Value> = serde_json::from_slice(&bytes).map_err(|e| {
        CompileError::new("E2002", format!("invalid manifest json: {e}"), manifest_line)
            .with_file(full.display().to_string())
    })?;

    let mut out = HashMap::new();
    for (k, v) in map {
        let Some(s) = v.as_str() else {
            return Err(
                CompileError::new("E2003", "manifest values must be strings", manifest_line)
                    .with_file(full.display().to_string()),
            );
        };
        if k.trim().is_empty() || s.trim().is_empty() {
            return Err(
                CompileError::new("E2003", "manifest keys/values must be non-empty", manifest_line)
                    .with_file(full.display().to_string()),
            );
        }
        out.insert(k, s.to_string());
    }
    Ok(out)
}
