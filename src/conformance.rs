use crate::{CONFORMANCE_SCHEMA, VERSION};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ConformanceOptions {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub output: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct ConformanceReport {
    pub schema: String,
    pub created_at: DateTime<Utc>,
    pub caleta_version: String,
    pub base_url: String,
    pub passed: bool,
    pub checks: Vec<Check>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Check {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

pub async fn conformance(options: ConformanceOptions) -> Result<ConformanceReport> {
    if !(options.base_url.starts_with("http://") || options.base_url.starts_with("https://")) {
        bail!("base URL must start with http:// or https://");
    }
    let parsed = reqwest::Url::parse(&options.base_url).context("invalid base URL")?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        bail!("base URL cannot contain credentials");
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        bail!("base URL cannot contain a query or fragment");
    }
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(120))
        .build()?;
    let base = options.base_url.trim_end_matches('/');
    let mut checks = Vec::new();

    let models = send(
        client.get(format!("{base}/v1/models")),
        options.api_key.as_deref(),
    )
    .await;
    let model_from_list = match &models {
        Ok(value) => value
            .pointer("/data/0/id")
            .and_then(Value::as_str)
            .map(str::to_string),
        Err(_) => None,
    };
    checks.push(check("models_list", &models, |value| {
        value.get("data").and_then(Value::as_array).is_some()
    }));
    let model = options
        .model
        .clone()
        .or(model_from_list)
        .unwrap_or_else(|| "default".into());

    let non_streaming = send(
        client
            .post(format!("{base}/v1/chat/completions"))
            .json(&chat_body(&model, false)),
        options.api_key.as_deref(),
    )
    .await;
    checks.push(check("chat_non_streaming", &non_streaming, |value| {
        value
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .is_some()
    }));

    let streaming = stream_check(&client, base, &model, options.api_key.as_deref()).await;
    match &streaming {
        Ok(detail) => checks.push(Check {
            name: "chat_streaming_sse".into(),
            passed: true,
            detail: detail.clone(),
        }),
        Err(error) => checks.push(Check {
            name: "chat_streaming_sse".into(),
            passed: false,
            detail: format!("{error:#}"),
        }),
    }

    let report = ConformanceReport {
        schema: CONFORMANCE_SCHEMA.into(),
        created_at: Utc::now(),
        caleta_version: VERSION.into(),
        base_url: base.into(),
        passed: checks.iter().all(|check| check.passed),
        checks,
        limitations: vec![
            "checks_a_small_openai_compatible_subset".into(),
            "does_not_test_model_quality_performance_or_safety".into(),
            "prompts_and_responses_are_not_included".into(),
        ],
    };
    if let Some(path) = &options.output {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        tokio::fs::write(path, serde_json::to_vec_pretty(&report)?)
            .await
            .with_context(|| format!("cannot write {}", path.display()))?;
    }
    Ok(report)
}

fn chat_body(model: &str, stream: bool) -> Value {
    json!({
        "model": model,
        "messages": [{"role":"user","content":"Reply with the single word: ok"}],
        "max_tokens": 8,
        "temperature": 0,
        "stream": stream
    })
}

async fn send(builder: reqwest::RequestBuilder, api_key: Option<&str>) -> Result<Value> {
    let builder = match api_key {
        Some(key) => builder.bearer_auth(key),
        None => builder,
    };
    let response = builder.send().await?;
    let status = response.status();
    let value: Value = response.json().await.context("response is not JSON")?;
    if !status.is_success() {
        bail!("HTTP {status}: error response body omitted");
    }
    Ok(value)
}

fn check(name: &str, result: &Result<Value>, predicate: impl FnOnce(&Value) -> bool) -> Check {
    match result {
        Ok(value) if predicate(value) => Check {
            name: name.into(),
            passed: true,
            detail: "valid response shape".into(),
        },
        Ok(_value) => Check {
            name: name.into(),
            passed: false,
            detail: "unexpected response shape; response body omitted".into(),
        },
        Err(error) => Check {
            name: name.into(),
            passed: false,
            detail: format!("{error:#}"),
        },
    }
}

async fn stream_check(
    client: &reqwest::Client,
    base: &str,
    model: &str,
    api_key: Option<&str>,
) -> Result<String> {
    let mut builder = client
        .post(format!("{base}/v1/chat/completions"))
        .json(&chat_body(model, true));
    if let Some(key) = api_key {
        builder = builder.bearer_auth(key);
    }
    let response = builder.send().await?;
    if !response.status().is_success() {
        bail!("streaming endpoint returned HTTP {}", response.status());
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .unwrap_or_default()
        .trim()
        .to_string();
    if content_type != "text/event-stream" {
        bail!("expected text/event-stream, received {content_type:?}");
    }
    let mut stream = response.bytes_stream();
    let mut text = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if text.len() + chunk.len() > 16 * 1024 * 1024 {
            bail!("SSE response exceeded the 16 MiB limit");
        }
        text.push_str(&String::from_utf8_lossy(&chunk));
    }
    validate_sse_events(&text)?;
    Ok("SSE content type, delta events and [DONE] observed".into())
}

fn validate_sse_events(text: &str) -> Result<()> {
    let normalized = text.replace("\r\n", "\n");
    let mut has_delta = false;
    let mut has_done = false;
    for event in normalized.split("\n\n") {
        let data = event
            .lines()
            .filter_map(|line| line.trim_start().strip_prefix("data:"))
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n");
        if data.is_empty() {
            continue;
        }
        if data == "[DONE]" {
            has_done = true;
            continue;
        }
        let value: Value = serde_json::from_str(&data).context("invalid SSE JSON event")?;
        if value
            .get("choices")
            .and_then(Value::as_array)
            .map(|choices| choices.iter().any(|choice| choice.get("delta").is_some()))
            .unwrap_or(false)
        {
            has_delta = true;
        }
    }
    if !has_done {
        bail!("stream did not contain an exact data: [DONE] event");
    }
    if !has_delta {
        bail!("stream did not contain an OpenAI-style delta object");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_structural_sse() {
        let stream = "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\ndata: [DONE]\n\n";
        assert!(validate_sse_events(stream).is_ok());
    }

    #[test]
    fn rejects_text_that_only_looks_like_sse() {
        assert!(validate_sse_events("garbage \"delta\" data: [DONE]").is_err());
    }

    #[test]
    fn rejects_invalid_json_event() {
        assert!(validate_sse_events("data: not-json\n\ndata: [DONE]\n\n").is_err());
    }

    #[test]
    fn omits_unexpected_response_bodies() {
        let result = check("privacy", &Ok(json!({"secret":"do-not-publish"})), |_| {
            false
        });
        assert!(!result.detail.contains("do-not-publish"));
    }
}
