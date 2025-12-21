use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
    path: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let chart = mdf_runner::load_chart_json_from_path(args.path)?;
    println!("title={}", chart.meta.title);
    println!("artist={}", chart.meta.artist);
    println!("version={}", chart.meta.version);
    println!("total_duration_us={}", chart.meta.total_duration_us);
    Ok(())
}
