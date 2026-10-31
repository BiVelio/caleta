use crate::backend::{Backend, BackendConfig};
use crate::manifest::LoadedManifest;
use crate::verify::verify_manifest;
use crate::{BENCHMARK_SCHEMA, VERSION};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct BenchOptions {
    pub runs: u32,
    pub warmup: u32,
    pub prompt: String,
    pub max_tokens: u32,
    pub seed: u64,
    pub include_prompt: bool,
    pub startup_timeout: Duration,
    pub output: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct BenchmarkReport {
    pub schema: String,
    pub created_at: DateTime<Utc>,
    pub caleta_version: String,
    pub model_id: String,
    pub model_sha256: String,
    pub backend_version: Option<String>,
    pub os: String,
    pub arch: String,
    pub cold_start_ms: f64,
    pub context_size: u32,
    pub gpu_layers: i32,
    pub threads: Option<u32>,
    pub max_tokens: u32,
    pub seed: u64,
    pub temperature: u32,
    pub warmup_runs: u32,
    pub prompt_sha256: String,
    pub prompt_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    pub runs: Vec<RunRecord>,
    pub summary: Option<Summary>,
    pub peak_backend_rss_kib: Option<u64>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RunRecord {
    pub run: u32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<RunMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RunMetrics {
    pub total_ms: f64,
    pub ttft_ms: f64,
    pub decode_tps: Option<f64>,
    pub observed_prefill_tps: Option<f64>,
    pub itl_p50_ms: Option<f64>,
    pub itl_p95_ms: Option<f64>,
    pub itl_p99_ms: Option<f64>,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: u64,
    pub token_count_estimated: bool,
    pub content_events: u64,
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub successful_runs: u32,
    pub failed_runs: u32,
    pub ttft_median_ms: f64,
    pub decode_tps_median: Option<f64>,
    pub itl_p50_ms: Option<f64>,
    pub itl_p95_ms: Option<f64>,
    pub itl_p99_ms: Option<f64>,
}

pub async fn benchmark(loaded: LoadedManifest, options: BenchOptions) -> Result<PathBuf> {
    if options.runs == 0 {
        bail!("runs must be greater than zero");
    }
    if options.max_tokens == 0 {
        bail!("max_tokens must be greater than zero");
    }
    let verification = verify_manifest(&loaded).await?;
    if !verification.verified {
        bail!("integrity verification failed; benchmark was not started");
    }
    let backend_version = Backend::version(&loaded.backend_path()).await;
    let mut backend = Backend::spawn(
        &loaded,
        BackendConfig {
            startup_timeout: options.startup_timeout,
        },
    )
    .await?;
    let cold_start_ms = milliseconds(backend.started_in);
    let (peak_rss, stop_monitor, monitor) = monitor_backend(backend.id());
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(120))
        .build()?;

    for index in 0..options.warmup {
        if let Err(error) = measure(&client, &backend, &loaded, &options).await {
            tracing::warn!(warmup = index + 1, %error, "warm-up request failed");
        }
    }

    let mut records = Vec::new();
    for run in 1..=options.runs {
        match measure(&client, &backend, &loaded, &options).await {
            Ok(metrics) => records.push(RunRecord {
                run,
                status: "ok".into(),
                metrics: Some(metrics),
                error: None,
            }),
            Err(error) => records.push(RunRecord {
                run,
                status: "error".into(),
                metrics: None,
                error: Some(format!("{error:#}")),
            }),
        }
    }

    if let Some(sender) = stop_monitor {
        sender.send(true).ok();
    }
    if let Some(handle) = monitor {
        handle.await.ok();
    }
    backend.shutdown().await?;
    let summary = summarize(&records);
    let failed = records
        .iter()
        .filter(|record| record.metrics.is_none())
        .count();
    let successful = records.len() - failed;
    let prompt_hash = hex::encode(Sha256::digest(options.prompt.as_bytes()));
    let report = BenchmarkReport {
        schema: BENCHMARK_SCHEMA.into(),
        created_at: Utc::now(),
        caleta_version: VERSION.into(),
        model_id: loaded.manifest.id.clone(),
        model_sha256: loaded.manifest.artifact.sha256.to_ascii_lowercase(),
        backend_version,
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        cold_start_ms,
        context_size: loaded.manifest.runtime.context_size,
        gpu_layers: loaded.manifest.runtime.gpu_layers,
        threads: loaded.manifest.runtime.threads,
        max_tokens: options.max_tokens,
        seed: options.seed,
        temperature: 0,
        warmup_runs: options.warmup,
        prompt_sha256: prompt_hash,
        prompt_bytes: options.prompt.len(),
        prompt: options.include_prompt.then(|| options.prompt.clone()),
        runs: records,
        summary,
        peak_backend_rss_kib: peak_rss.map(|value| value.load(Ordering::Relaxed)),
        limitations: vec![
            "single_process_warm_runs_after_one_cold_start".into(),
            "ttft_and_prefill_include_http_queueing_overhead".into(),
            "itl_is_measured_between_content_bearing_sse_events_not_tokenizer_steps".into(),
            "rss_covers_the_main_backend_process_and_excludes_gpu_memory".into(),
            "energy_and_quantization_quality_are_not_measured".into(),
        ],
    };
    if let Some(parent) = options.output.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    tokio::fs::write(&options.output, serde_json::to_vec_pretty(&report)?)
        .await
        .with_context(|| format!("cannot write {}", options.output.display()))?;
    println!(
        "Benchmark: {successful} ok, {failed} failed; report: {}",
        options.output.display()
    );
    if failed > 0 {
        bail!("{failed} benchmark requests failed; raw failures were preserved in the report");
    }
    Ok(options.output)
}

async fn measure(
    client: &reqwest::Client,
    backend: &Backend,
    loaded: &LoadedManifest,
    options: &BenchOptions,
) -> Result<RunMetrics> {
    let started = Instant::now();
    let response = client
        .post(format!("{}/v1/chat/completions", backend.base_url))
        .bearer_auth(&backend.api_key)
        .json(&json!({
            "model": loaded.manifest.id,
            "messages": [{"role":"user","content":options.prompt}],
            "max_tokens": options.max_tokens,
            "temperature": 0,
            "seed": options.seed,
            "stream": true,
            "stream_options": {"include_usage": true}
        }))
        .send()
        .await?;
    if !response.status().is_success() {
        let status = response.status();
        bail!("backend returned {status}; response body omitted");
    }

    let mut stream = response.bytes_stream();
    let mut buffer = Vec::<u8>::new();
    let mut first_content: Option<Instant> = None;
    let mut previous_content: Option<Instant> = None;
    let mut last_content: Option<Instant> = None;
    let mut intervals = Vec::new();
    let mut content_events = 0_u64;
    let mut prompt_tokens = None;
    let mut usage_completion_tokens = None;

    while let Some(chunk) = stream.next().await {
        buffer.extend_from_slice(&chunk?);
        while let Some((position, length)) = find_event_boundary(&buffer) {
            let event: Vec<u8> = buffer.drain(..position + length).collect();
            let now = Instant::now();
            let observation = parse_event(&event)?;
            if observation.has_content {
                if first_content.is_none() {
                    first_content = Some(now);
                }
                if let Some(previous) = previous_content {
                    intervals.push(milliseconds(now.duration_since(previous)));
                }
                previous_content = Some(now);
                last_content = Some(now);
                content_events += 1;
            }
            if observation.prompt_tokens.is_some() {
                prompt_tokens = observation.prompt_tokens;
            }
            if observation.completion_tokens.is_some() {
                usage_completion_tokens = observation.completion_tokens;
            }
        }
    }

    let first = first_content.context("stream ended without content")?;
    let ttft_ms = milliseconds(first.duration_since(started));
    let completion_tokens = usage_completion_tokens.unwrap_or(content_events);
    let token_count_estimated = usage_completion_tokens.is_none();
    let decode_tps = match (last_content, completion_tokens > 1) {
        (Some(last), true) => {
            let seconds = last.duration_since(first).as_secs_f64();
            (seconds > 0.0).then_some((completion_tokens - 1) as f64 / seconds)
        }
        _ => None,
    };
    let observed_prefill_tps = prompt_tokens.and_then(|tokens| {
        let seconds = Duration::from_secs_f64(ttft_ms / 1000.0).as_secs_f64();
        (seconds > 0.0).then_some(tokens as f64 / seconds)
    });

    Ok(RunMetrics {
        total_ms: milliseconds(started.elapsed()),
        ttft_ms,
        decode_tps,
        observed_prefill_tps,
        itl_p50_ms: percentile(&intervals, 0.50),
        itl_p95_ms: percentile(&intervals, 0.95),
        itl_p99_ms: percentile(&intervals, 0.99),
        prompt_tokens,
        completion_tokens,
        token_count_estimated,
        content_events,
    })
}

#[derive(Default)]
struct Observation {
    has_content: bool,
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
}

fn parse_event(event: &[u8]) -> Result<Observation> {
    let text = String::from_utf8_lossy(event);
    let data = text
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("data:"))
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n");
    if data.is_empty() || data == "[DONE]" {
        return Ok(Observation::default());
    }
    let value: Value = serde_json::from_str(&data).context("invalid SSE JSON event")?;
    let has_content = value
        .get("choices")
        .and_then(Value::as_array)
        .map(|choices| {
            choices.iter().any(|choice| {
                choice
                    .pointer("/delta/content")
                    .and_then(Value::as_str)
                    .map(|content| !content.is_empty())
                    .unwrap_or(false)
                    || choice
                        .pointer("/delta/reasoning_content")
                        .and_then(Value::as_str)
                        .map(|content| !content.is_empty())
                        .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    Ok(Observation {
        has_content,
        prompt_tokens: value
            .pointer("/usage/prompt_tokens")
            .and_then(Value::as_u64),
        completion_tokens: value
            .pointer("/usage/completion_tokens")
            .and_then(Value::as_u64),
    })
}

fn find_event_boundary(buffer: &[u8]) -> Option<(usize, usize)> {
    (0..buffer.len()).find_map(|index| {
        if buffer[index..].starts_with(b"\r\n\r\n") {
            Some((index, 4))
        } else if buffer[index..].starts_with(b"\n\n") {
            Some((index, 2))
        } else {
            None
        }
    })
}

fn summarize(records: &[RunRecord]) -> Option<Summary> {
    let metrics = records
        .iter()
        .filter_map(|record| record.metrics.as_ref())
        .collect::<Vec<_>>();
    if metrics.is_empty() {
        return None;
    }
    let ttft = metrics.iter().map(|item| item.ttft_ms).collect::<Vec<_>>();
    let decode = metrics
        .iter()
        .filter_map(|item| item.decode_tps)
        .collect::<Vec<_>>();
    Some(Summary {
        successful_runs: metrics.len() as u32,
        failed_runs: (records.len() - metrics.len()) as u32,
        ttft_median_ms: percentile(&ttft, 0.50)?,
        decode_tps_median: percentile(&decode, 0.50),
        itl_p50_ms: percentile(
            &metrics
                .iter()
                .filter_map(|item| item.itl_p50_ms)
                .collect::<Vec<_>>(),
            0.50,
        ),
        itl_p95_ms: percentile(
            &metrics
                .iter()
                .filter_map(|item| item.itl_p95_ms)
                .collect::<Vec<_>>(),
            0.50,
        ),
        itl_p99_ms: percentile(
            &metrics
                .iter()
                .filter_map(|item| item.itl_p99_ms)
                .collect::<Vec<_>>(),
            0.50,
        ),
    })
}

fn percentile(values: &[f64], quantile: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut values = values.to_vec();
    values.sort_by(f64::total_cmp);
    let position = quantile * (values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let weight = position - lower as f64;
    Some(values[lower] * (1.0 - weight) + values[upper] * weight)
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

type BackendMonitor = (
    Option<Arc<AtomicU64>>,
    Option<tokio::sync::watch::Sender<bool>>,
    Option<tokio::task::JoinHandle<()>>,
);

fn monitor_backend(pid: Option<u32>) -> BackendMonitor {
    #[cfg(not(target_os = "linux"))]
    let _ = pid;
    #[cfg(target_os = "linux")]
    if let Some(pid) = pid {
        let peak = Arc::new(AtomicU64::new(0));
        let output = peak.clone();
        let (sender, mut receiver) = tokio::sync::watch::channel(false);
        let handle = tokio::spawn(async move {
            loop {
                if let Ok(status) = tokio::fs::read_to_string(format!("/proc/{pid}/status")).await {
                    if let Some(kib) = status
                        .lines()
                        .find_map(|line| line.strip_prefix("VmRSS:"))
                        .and_then(|line| line.split_whitespace().next())
                        .and_then(|value| value.parse::<u64>().ok())
                    {
                        peak.fetch_max(kib, Ordering::Relaxed);
                    }
                }
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {},
                    _ = receiver.changed() => break,
                }
            }
        });
        return (Some(output), Some(sender), Some(handle));
    }
    #[allow(unreachable_code)]
    (None, None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_content_and_usage() {
        let event = br#"data: {"choices":[{"delta":{"content":"hola"}}],"usage":{"prompt_tokens":4,"completion_tokens":2}}

"#;
        let observation = parse_event(event).unwrap();
        assert!(observation.has_content);
        assert_eq!(observation.prompt_tokens, Some(4));
        assert_eq!(observation.completion_tokens, Some(2));
    }

    #[test]
    fn ignores_done() {
        assert!(!parse_event(b"data: [DONE]\n\n").unwrap().has_content);
    }

    #[test]
    fn computes_percentiles() {
        assert_eq!(percentile(&[1.0, 2.0, 3.0], 0.5), Some(2.0));
        assert_eq!(percentile(&[], 0.5), None);
    }
}
