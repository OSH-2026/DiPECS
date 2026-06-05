//! Rust llama-compatible experiment CLI.

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use lab4_tools::jsonl::write_jsonl_record;
use lab4_tools::llama::{
    build_bench_record, unix_time_ms, GenerationConfig, LlamaBenchMeta, LlamaBenchRecord,
    LlamaResponse, LoadMode, LocalLlama,
};
use lab4_tools::prompt::load_prompts;
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(
    name = "lab4-llama",
    about = "Rust llama-compatible Lab4 experiment runner"
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run one local inference request.
    Infer(InferArgs),
    /// Run a prompt JSONL benchmark locally.
    Bench(BenchArgs),
    /// Run a sequential TCP RPC worker.
    RpcWorker(WorkerArgs),
    /// Send one request to a TCP RPC worker.
    RpcMaster(MasterArgs),
}

#[derive(Debug, Parser)]
struct InferArgs {
    /// Model file path.
    #[arg(long)]
    model: PathBuf,

    /// Prompt text.
    #[arg(long)]
    prompt: String,

    /// Model loading mode.
    #[arg(long, value_enum, default_value_t = CliLoadMode::Mmap)]
    load_mode: CliLoadMode,

    /// Maximum generated token estimate.
    #[arg(long, default_value_t = 128)]
    max_tokens: usize,

    /// Requested CPU thread count.
    #[arg(long, default_value_t = 1)]
    threads: usize,

    /// Requested batch size.
    #[arg(long, default_value_t = 1)]
    batch_size: usize,

    /// Requested context window size.
    #[arg(long, default_value_t = 512)]
    ctx_size: usize,
}

#[derive(Debug, Parser)]
struct BenchArgs {
    /// Model file path.
    #[arg(long)]
    model: PathBuf,

    /// Prompt JSONL path.
    #[arg(long)]
    prompts: PathBuf,

    /// Output JSONL path.
    #[arg(long)]
    output: PathBuf,

    /// Experiment mode label.
    #[arg(long, default_value = "single")]
    mode: String,

    /// Case id prefix.
    #[arg(long, default_value = "rust-llama")]
    case_prefix: String,

    /// Model loading mode.
    #[arg(long, value_enum, default_value_t = CliLoadMode::Mmap)]
    load_mode: CliLoadMode,

    /// Default max tokens when prompt omits max_tokens.
    #[arg(long, default_value_t = 128)]
    max_tokens: usize,

    /// Requested CPU thread count.
    #[arg(long, default_value_t = 1)]
    threads: usize,

    /// Requested batch size.
    #[arg(long, default_value_t = 1)]
    batch_size: usize,

    /// Requested context window size.
    #[arg(long, default_value_t = 512)]
    ctx_size: usize,
}

#[derive(Debug, Parser)]
struct WorkerArgs {
    /// TCP address to bind, for example 0.0.0.0:50052.
    #[arg(long, default_value = "127.0.0.1:50052")]
    bind: String,

    /// Model file path.
    #[arg(long)]
    model: PathBuf,

    /// Model loading mode.
    #[arg(long, value_enum, default_value_t = CliLoadMode::Mmap)]
    load_mode: CliLoadMode,
}

#[derive(Debug, Parser)]
struct MasterArgs {
    /// Worker endpoint, for example 127.0.0.1:50052.
    #[arg(long)]
    endpoint: String,

    /// Prompt text.
    #[arg(long)]
    prompt: String,

    /// Maximum generated token estimate.
    #[arg(long, default_value_t = 128)]
    max_tokens: usize,

    /// Requested CPU thread count.
    #[arg(long, default_value_t = 1)]
    threads: usize,

    /// Requested batch size.
    #[arg(long, default_value_t = 1)]
    batch_size: usize,

    /// Requested context window size.
    #[arg(long, default_value_t = 512)]
    ctx_size: usize,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliLoadMode {
    Mmap,
    Read,
}

#[derive(Debug, Deserialize, Serialize)]
struct RpcRequest {
    prompt: String,
    config: GenerationConfig,
}

#[derive(Debug, Deserialize, Serialize)]
struct RpcResponse {
    response: LlamaResponse,
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::Infer(args) => run_infer(args),
        Command::Bench(args) => run_bench(args),
        Command::RpcWorker(args) => run_worker(args),
        Command::RpcMaster(args) => run_master(args),
    }
}

fn run_infer(args: InferArgs) -> Result<()> {
    let config = GenerationConfig {
        max_tokens: args.max_tokens,
        threads: args.threads,
        batch_size: args.batch_size,
        ctx_size: args.ctx_size,
        load_mode: args.load_mode.into(),
    };
    let started = Instant::now();
    let llama = LocalLlama::load(&args.model, config.load_mode)
        .with_context(|| format!("loading model {}", args.model.display()))?;
    let load_duration_ms = started.elapsed().as_millis();
    let response = llama.infer(&args.prompt, &config);

    println!("{}", response.text);
    eprintln!(
        "model={} bytes={} loaded_storage_bytes={} fingerprint={:016x} load_ms={} infer_ms={} tokens_per_second={}",
        llama.model_path().display(),
        llama.model_bytes(),
        llama.loaded_storage_bytes(),
        llama.model_fingerprint(),
        load_duration_ms,
        response.inference_duration_ms,
        response
            .tokens_per_second
            .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.2}"))
    );
    Ok(())
}

fn run_bench(args: BenchArgs) -> Result<()> {
    let prompts = load_prompts(&args.prompts)
        .with_context(|| format!("loading prompts {}", args.prompts.display()))?;
    let output = File::create(&args.output)
        .with_context(|| format!("creating output {}", args.output.display()))?;
    let mut writer = BufWriter::new(output);
    let load_mode = args.load_mode.into();

    for (index, prompt) in prompts.iter().enumerate() {
        let started_at_unix_ms = unix_time_ms()?;
        let total_started = Instant::now();
        let load_started = Instant::now();
        let llama = LocalLlama::load(&args.model, load_mode)
            .with_context(|| format!("loading model {}", args.model.display()))?;
        let load_duration_ms = load_started.elapsed().as_millis();
        let config = GenerationConfig {
            max_tokens: prompt
                .max_tokens
                .and_then(|tokens| usize::try_from(tokens).ok())
                .unwrap_or(args.max_tokens),
            threads: args.threads,
            batch_size: args.batch_size,
            ctx_size: args.ctx_size,
            load_mode,
        };
        let response = llama.infer(&prompt.prompt, &config);
        let mut record = build_bench_record(
            LlamaBenchMeta {
                case_id: format!("{}-{:03}", args.case_prefix, index + 1),
                prompt_id: prompt.id.clone(),
                category: prompt.category.clone(),
                mode: args.mode.clone(),
                load_duration_ms,
                started_at_unix_ms,
            },
            &llama,
            &config,
            response,
        );
        record.total_duration_ms = total_started.elapsed().as_millis();
        write_jsonl_record(&mut writer, &record)?;
        writer.flush()?;
    }
    Ok(())
}

fn run_worker(args: WorkerArgs) -> Result<()> {
    let load_mode = args.load_mode.into();
    let llama = LocalLlama::load(&args.model, load_mode)
        .with_context(|| format!("loading model {}", args.model.display()))?;
    let listener =
        TcpListener::bind(&args.bind).with_context(|| format!("binding {}", args.bind))?;
    eprintln!(
        "lab4-llama rpc-worker listening on {} model={} bytes={}",
        args.bind,
        llama.model_path().display(),
        llama.model_bytes()
    );

    for stream in listener.incoming() {
        let stream = stream.with_context(|| format!("accepting connection on {}", args.bind))?;
        handle_worker_stream(stream, &llama)?;
    }
    Ok(())
}

fn handle_worker_stream(stream: TcpStream, llama: &LocalLlama) -> Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let request: RpcRequest = serde_json::from_str(&request_line)?;
    let response = RpcResponse {
        response: llama.infer(&request.prompt, &request.config),
    };
    let mut writer = BufWriter::new(stream);
    serde_json::to_writer(&mut writer, &response)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

fn run_master(args: MasterArgs) -> Result<()> {
    let config = GenerationConfig {
        max_tokens: args.max_tokens,
        threads: args.threads,
        batch_size: args.batch_size,
        ctx_size: args.ctx_size,
        load_mode: LoadMode::Mmap,
    };
    let request = RpcRequest {
        prompt: args.prompt,
        config,
    };
    let mut stream = TcpStream::connect(&args.endpoint)
        .with_context(|| format!("connecting {}", args.endpoint))?;
    serde_json::to_writer(&mut stream, &request)?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;
    let response: RpcResponse = serde_json::from_str(&response_line)?;
    println!("{}", response.response.text);
    eprintln!(
        "remote_infer_ms={} tokens_per_second={}",
        response.response.inference_duration_ms,
        response
            .response
            .tokens_per_second
            .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.2}"))
    );
    Ok(())
}

impl From<CliLoadMode> for LoadMode {
    fn from(value: CliLoadMode) -> Self {
        match value {
            CliLoadMode::Mmap => Self::Mmap,
            CliLoadMode::Read => Self::Read,
        }
    }
}

#[allow(dead_code)]
fn _assert_record_is_serializable(record: &LlamaBenchRecord) -> Result<String> {
    Ok(serde_json::to_string(record)?)
}
