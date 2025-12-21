use std::{fs, path::Path};

use anyhow::Context;
use mdf_schema::MdfChart;

pub fn load_chart_json_from_path(path: impl AsRef<Path>) -> anyhow::Result<MdfChart> {
    let path = path.as_ref();
    let bytes = fs::read(path).with_context(|| format!("failed to read chart: {}", path.display()))?;
    let chart: MdfChart = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse chart json: {}", path.display()))?;
    Ok(chart)
}

pub fn load_chart_json_from_str(json: &str) -> anyhow::Result<MdfChart> {
    let chart: MdfChart = serde_json::from_str(json).context("failed to parse chart json")?;
    Ok(chart)
}
