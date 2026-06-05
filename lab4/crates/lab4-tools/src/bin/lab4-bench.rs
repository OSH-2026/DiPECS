//! llama.cpp benchmark runner CLI.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use lab4_tools::command::{run_llama_case, LlamaRunConfig};
use lab4_tools::jsonl::write_jsonl_record;
use lab4_tools::prompt::load_prompts;

#[derive(Debug, Parser)]
#[command(
    name = "lab4-bench",
    about = "Run llama.cpp prompts and emit JSONL metrics"
)]
struct Args {
    /// Prompt JSONL path.
    #[arg(long)]
    prompts: PathBuf,

    /// llama.cpp executable path, usually llama-cli.
    #[arg(long)]
    executable: PathBuf,

    /// GGUF model path.
    #[arg(long)]
    model: PathBuf,

    /// Output JSONL path.
    #[arg(long)]
    output: PathBuf,

    /// Experiment mode label, for example single or rpc.
    #[arg(long, default_value = "single")]
    mode: String,

    /// Optional RPC endpoint list for llama.cpp --rpc.
    #[arg(long)]
    rpc: Option<String>,

    /// Optional command working directory.
    #[arg(long)]
    working_dir: Option<PathBuf>,

    /// Default token limit when a prompt record omits max_tokens.
    #[arg(long)]
    max_tokens: Option<u32>,

    /// Prefix for generated case ids.
    #[arg(long, default_value = "case")]
    case_prefix: String,

    /// Extra llama.cpp argument. Repeat as --arg=--threads --arg=8.
    #[arg(long = "arg")]
    extra_args: Vec<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let prompts = load_prompts(&args.prompts)
        .with_context(|| format!("loading prompts {}", args.prompts.display()))?;
    let output = File::create(&args.output)
        .with_context(|| format!("creating output {}", args.output.display()))?;
    let mut writer = BufWriter::new(output);
    let config = LlamaRunConfig {
        executable: args.executable,
        model: args.model,
        working_dir: args.working_dir,
        mode: args.mode,
        rpc_endpoints: args.rpc,
        default_max_tokens: args.max_tokens,
        extra_args: args.extra_args,
    };

    for (index, prompt) in prompts.iter().enumerate() {
        let case_id = format!("{}-{:03}", args.case_prefix, index + 1);
        let record = run_llama_case(&config, prompt, case_id)?;
        write_jsonl_record(&mut writer, &record)?;
        writer.flush()?;
    }
    Ok(())
}
