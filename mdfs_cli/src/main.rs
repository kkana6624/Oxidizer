use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use clap::{Parser, Subcommand};

mod simulate;

#[derive(Debug, Parser)]
#[command(name = "mdfs")]
#[command(about = "MDFS compiler CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Compile {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Simulate {
        input: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Compile { input, output } => {
            let chart = mdfs_compiler::compile_file(&input)
                .map_err(|e| anyhow::anyhow!(e.to_string()))
                .with_context(|| format!("compile failed: {}", input.display()))?;

            let json = serde_json::to_string_pretty(&chart).context("failed to serialize mdf")?;
            let out_path = output.unwrap_or_else(|| default_output_path(&input));
            fs::write(&out_path, json)
                .with_context(|| format!("failed to write: {}", out_path.display()))?;
        }
        Command::Simulate { input } => {
            // If input is .mdfs, compile first. If .json, load directly.
            let chart = if input.extension().map_or(false, |e| e == "mdfs") {
                mdfs_compiler::compile_file(&input)
                    .map_err(|e| anyhow::anyhow!(e.to_string()))
                    .with_context(|| format!("compile failed: {}", input.display()))?
            } else {
                let bytes = fs::read(&input)
                    .with_context(|| format!("failed to read: {}", input.display()))?;
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("failed to parse json: {}", input.display()))?
            };
            simulate::run_simulation(&chart)?;
        }
    }

    Ok(())
}

fn default_output_path(input: &Path) -> PathBuf {
    let mut out = input.to_path_buf();
    out.set_extension("mdf.json");
    out
}
