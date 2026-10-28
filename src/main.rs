use anyhow::{bail, Result};
use caleta::bench::{benchmark, BenchOptions};
use caleta::conformance::{conformance, ConformanceOptions};
use caleta::doctor::doctor;
use caleta::manifest::LoadedManifest;
use caleta::server::{serve, ServeOptions};
use caleta::verify::verify_manifest;
use clap::{Args, Parser, Subcommand};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "caleta", version, about = "Private local AI, with evidence.")]
#[command(
    long_about = "Caleta verifies model artifacts, supervises llama.cpp, and exposes a safer OpenAI-compatible local API. It does not implement inference kernels."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Verify the model artifact and, when pinned, the backend binary.
    Verify {
        manifest: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Verify a model and serve an OpenAI-compatible API.
    #[command(visible_alias = "run")]
    Serve(ServeArgs),
    /// Check local runtime prerequisites without sending model data anywhere.
    Doctor {
        #[arg(long)]
        manifest: Option<PathBuf>,
        #[arg(long)]
        backend: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Run reproducible warm inference measurements and preserve raw results.
    Bench(BenchArgs),
    /// Test a small OpenAI-compatible API subset against any local server.
    Conformance(ConformanceArgs),
}

#[derive(Args)]
struct ServeArgs {
    manifest: PathBuf,
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: SocketAddr,
    /// Name of the environment variable containing the public API key.
    #[arg(long, default_value = "CALETA_API_KEY")]
    api_key_env: String,
    /// Explicit browser origin allowed by CORS; repeat for multiple origins.
    #[arg(long = "cors-origin")]
    cors_origins: Vec<String>,
    #[arg(long, default_value_t = 120)]
    startup_timeout_secs: u64,
    /// Optional prompt-free execution evidence JSON.
    #[arg(long)]
    record_out: Option<PathBuf>,
}

#[derive(Args)]
struct BenchArgs {
    manifest: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 3)]
    runs: u32,
    #[arg(long, default_value_t = 1)]
    warmup: u32,
    #[arg(
        long,
        default_value = "Explain local AI privacy in one concise paragraph."
    )]
    prompt: String,
    #[arg(long, default_value_t = 64)]
    max_tokens: u32,
    #[arg(long, default_value_t = 42)]
    seed: u64,
    /// Include prompt text in the output; by default only its hash is stored.
    #[arg(long)]
    include_prompt: bool,
    #[arg(long, default_value_t = 120)]
    startup_timeout_secs: u64,
}

#[derive(Args)]
struct ConformanceArgs {
    /// Server root, for example http://127.0.0.1:8080
    #[arg(long)]
    url: String,
    #[arg(long)]
    model: Option<String>,
    #[arg(long, default_value = "CALETA_API_KEY")]
    api_key_env: String,
    #[arg(long)]
    output: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("caleta=info")),
        )
        .with_target(false)
        .compact()
        .init();
    match execute(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

async fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Verify { manifest, json } => {
            let loaded = LoadedManifest::load(manifest)?;
            let report = verify_manifest(&loaded).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("Model: {}", report.model);
                println!(
                    "Artifact: {:?} ({})",
                    report.artifact.status,
                    report.artifact.path.display()
                );
                println!("Backend pinned: {}", report.backend_pinned);
                println!(
                    "Scope: integrity only; manifest authenticity must be established separately"
                );
            }
            if !report.verified {
                bail!("verification failed");
            }
        }
        Command::Serve(args) => {
            let api_key = read_key(&args.api_key_env)?;
            let loaded = LoadedManifest::load(args.manifest)?;
            serve(
                loaded,
                ServeOptions {
                    listen: args.listen,
                    api_key,
                    cors_origins: args.cors_origins,
                    startup_timeout: Duration::from_secs(args.startup_timeout_secs),
                    record_out: args.record_out,
                },
            )
            .await?;
        }
        Command::Doctor {
            manifest,
            backend,
            json,
        } => {
            let report = doctor(manifest, backend).await;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                for check in &report.checks {
                    println!(
                        "[{}] {}: {}",
                        if check.passed { "ok" } else { "fail" },
                        check.name,
                        check.detail
                    );
                }
            }
            if !report.healthy {
                bail!("doctor found unmet prerequisites");
            }
        }
        Command::Bench(args) => {
            let loaded = LoadedManifest::load(args.manifest)?;
            benchmark(
                loaded,
                BenchOptions {
                    runs: args.runs,
                    warmup: args.warmup,
                    prompt: args.prompt,
                    max_tokens: args.max_tokens,
                    seed: args.seed,
                    include_prompt: args.include_prompt,
                    startup_timeout: Duration::from_secs(args.startup_timeout_secs),
                    output: args.output,
                },
            )
            .await?;
        }
        Command::Conformance(args) => {
            let report = conformance(ConformanceOptions {
                base_url: args.url,
                api_key: read_key(&args.api_key_env)?,
                model: args.model,
                output: args.output,
            })
            .await?;
            for check in &report.checks {
                println!(
                    "[{}] {}: {}",
                    if check.passed { "ok" } else { "fail" },
                    check.name,
                    check.detail
                );
            }
            if !report.passed {
                bail!("conformance checks failed");
            }
        }
    }
    Ok(())
}

fn read_key(variable: &str) -> Result<Option<String>> {
    match std::env::var(variable) {
        Ok(value) if value.is_empty() => bail!("{variable} is set but empty"),
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => bail!("cannot read {variable}: {error}"),
    }
}
