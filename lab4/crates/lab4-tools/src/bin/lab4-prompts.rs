//! Prompt dataset validation CLI.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use lab4_tools::prompt::{load_prompts, summarize_prompts};

#[derive(Debug, Parser)]
#[command(name = "lab4-prompts", about = "Validate a Lab4 prompt JSONL file")]
struct Args {
    /// Prompt JSONL path.
    path: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let prompts = load_prompts(&args.path)?;
    let summary = summarize_prompts(&prompts);

    println!("prompts: {}", summary.total);
    for (category, count) in summary.categories {
        println!("{category}: {count}");
    }
    Ok(())
}
