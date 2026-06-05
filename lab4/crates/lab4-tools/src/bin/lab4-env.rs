//! Environment capture CLI for Lab4 deployment reports.

use anyhow::Result;
use lab4_tools::env::capture_environment;

fn main() -> Result<()> {
    let environment = capture_environment();
    serde_json::to_writer_pretty(std::io::stdout().lock(), &environment)?;
    println!();
    Ok(())
}
