# Benchmarking and conformance

## Reproducibility contract

A useful comparison must hold the artifact, quantization, prompt, maximum output budget, seed, temperature, context, backend build, and hardware constant. Caleta records these values when available and does not silently discard failed runs. Each benchmark performs one backend cold start, the requested warm-up requests, and measured warm requests in that same process. A cold-start measurement is reported separately; the measured runs are not presented as cold starts.

```bash
caleta bench model.toml \
  --output results/q4-baseline.json \
  --runs 5 --warmup 1 \
  --prompt 'Explain local AI privacy in one concise paragraph.' \
  --max-tokens 64 --seed 42
```

The default output contains the SHA-256 and byte length of the prompt, not its text. `--include-prompt` deliberately stores the prompt to improve standalone reproducibility. Responses and raw SSE bodies are never included. Per-run metrics and error strings are retained so that a failed configuration can be diagnosed instead of disappearing from an average.

## Metric definitions

| Metric | Definition | Important limitation |
|---|---|---|
| `cold_start_ms` | Process launch through authenticated health readiness | Includes process and model initialization but not Caleta build time |
| `ttft_ms` | Request send to first SSE event containing non-empty content or reasoning content | Includes HTTP and queueing overhead; empty role-only events do not count |
| `observed_prefill_tps` | Reported prompt tokens divided by TTFT | Not a pure kernel prefill measurement |
| `decode_tps` | `(completion_tokens - 1) / (last_content_time - first_content_time)` | Requires at least two tokens and content events; depends on backend chunking |
| `itl_p50/p95/p99_ms` | Percentiles of arrival intervals between content-bearing SSE events | Events may not map one-to-one to tokenizer tokens |
| `completion_tokens` | Backend usage when present; content-event count otherwise | `token_count_estimated` identifies the fallback |
| `peak_backend_rss_kib` | Highest sampled `/proc/PID/status` VmRSS, about every 50 ms on Linux | Main process only; excludes GPU memory and can miss short peaks |

Summary TTFT and decode values are medians across successful runs. Summary ITL values are medians of per-run percentiles, not percentiles over a pooled event distribution. Missing values remain JSON `null`; Caleta does not replace unsupported measurements with zero.

## Comparing engines fairly

Run each engine with the same model weights and quantization semantics, not merely similarly named files. Keep prompt templates and generation options equivalent, record immutable source revisions, and use the same hardware power state. Separate cold from warm measurements and publish all raw JSON. Add memory, energy, throttling, and output-quality evaluations only with instruments and methods appropriate to the platform. A single tokens/s number is insufficient to establish that one engine is “better.”

Caleta v0.1 does not implement energy measurement, GPU memory accounting, quantization-quality scoring, concurrency sweeps, or independent cold starts for every run. These are explicit extensions, not fields to populate with guesses.

## Conformance scope

```bash
caleta conformance --url http://127.0.0.1:8080 --output conformance.json
```

The suite checks that `/v1/models` returns a data array, non-streaming chat returns an OpenAI-style message content field, and streaming chat uses `text/event-stream`, emits delta events, and terminates with `data: [DONE]`. A model can be selected with `--model`; otherwise the first listed model or `default` is used. Credentials are read from `CALETA_API_KEY` or another named environment variable. URLs containing embedded credentials are rejected.

Passing these checks means only that this small contract passed. It does not certify full OpenAI compatibility, streaming correctness under fragmentation, tool calls, structured outputs, multimodal inputs, model quality, performance, or safety. The mock-based E2E test exercises the implementation but is not evidence of real-model accuracy or speed.
