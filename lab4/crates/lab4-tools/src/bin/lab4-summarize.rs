//! Benchmark summary CLI.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use lab4_tools::stats::summarize_bench_file;

#[derive(Debug, Parser)]
#[command(name = "lab4-summarize", about = "Summarize Lab4 benchmark JSONL")]
struct Args {
    /// Benchmark JSONL path.
    path: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let summaries = summarize_bench_file(&args.path)?;

    println!("| Mode | Records | Successes | Avg Duration ms | Avg tokens/s |");
    println!("| :--- | ---: | ---: | ---: | ---: |");
    for summary in summaries {
        let tokens_per_second = summary
            .average_tokens_per_second
            .map_or_else(|| "-".to_owned(), |value| format!("{value:.2}"));
        println!(
            "| {} | {} | {} | {:.2} | {} |",
            summary.mode,
            summary.records,
            summary.successes,
            summary.average_duration_ms,
            tokens_per_second
        );
    }
    Ok(())
}
