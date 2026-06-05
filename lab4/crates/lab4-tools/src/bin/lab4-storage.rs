//! Storage benchmark CLI for local and Ceph-backed paths.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use lab4_tools::jsonl::write_jsonl_record;
use lab4_tools::storage::{measure_copy, measure_read};

#[derive(Debug, Parser)]
#[command(name = "lab4-storage", about = "Measure file read/copy cost for Lab4")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Measure sequential read of one file.
    Read {
        /// Stable case id.
        #[arg(long)]
        case_id: String,

        /// Source path to read.
        path: PathBuf,

        /// Optional JSONL output path. Defaults to stdout.
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Measure file copy from source to target.
    Copy {
        /// Stable case id.
        #[arg(long)]
        case_id: String,

        /// Source path.
        source: PathBuf,

        /// Target path.
        target: PathBuf,

        /// Optional JSONL output path. Defaults to stdout.
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::Read {
            case_id,
            path,
            output,
        } => {
            let record = measure_read(case_id, &path)?;
            write_record(output, &record)
        },
        Command::Copy {
            case_id,
            source,
            target,
            output,
        } => {
            let record = measure_copy(case_id, &source, &target)?;
            write_record(output, &record)
        },
    }
}

fn write_record(
    output: Option<PathBuf>,
    record: &lab4_tools::storage::StorageRecord,
) -> Result<()> {
    match output {
        Some(path) => {
            let file = File::create(&path)
                .with_context(|| format!("creating storage output {}", path.display()))?;
            let mut writer = BufWriter::new(file);
            write_jsonl_record(&mut writer, record)?;
            writer.flush()?;
        },
        None => {
            let stdout = std::io::stdout();
            let mut writer = BufWriter::new(stdout.lock());
            write_jsonl_record(&mut writer, record)?;
            writer.flush()?;
        },
    }
    Ok(())
}
