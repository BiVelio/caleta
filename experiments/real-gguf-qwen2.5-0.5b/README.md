# Real-GGUF validation: Qwen2.5-0.5B-Instruct on Windows CPU

This directory contains the public evidence for Caleta's first real-model validation. It is a single-host software and measurement experiment, not a cross-engine benchmark, model-quality evaluation, or claim of universal performance.

## Outcome

- Publisher size and SHA-256 matched for the GGUF and pinned llama.cpp archive/executable.
- `doctor` and `verify` passed with the real artifacts.
- A real `llama-server` started through Caleta and answered model-list, non-streaming chat, and SSE streaming requests.
- The three conformance checks passed.
- Three independent benchmark invocations completed **15/15 measured warm requests with zero failures**.
- No model, backend binary, credentials, private prompts, or proprietary data are committed.

## Fixed artifacts

| Component | Version and identity |
|---|---|
| Caleta | v0.1.0; executable built from source revision `539ed3e` |
| Model | Qwen/Qwen2.5-0.5B-Instruct-GGUF, revision `9217f5db79a29953eb74d5343926648285ec7e67` |
| Model file | `qwen2.5-0.5b-instruct-q4_k_m.gguf` |
| Model license | Apache-2.0; see `QWEN-LICENSE.txt` |
| Model bytes | 491,400,032 |
| Model SHA-256 | `74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db` |
| Backend | llama.cpp `b11981`, commit `da6c30ade8c30930dbf49fe57f9f02d727e99b15` |
| Backend asset | `llama-b11981-bin-win-cpu-x64.zip` |
| Asset SHA-256 | `626dd4136947fb9cdf755523fefcad3d414858a4beca06ce424fcd3ee0a948c9` |
| Executable SHA-256 | `8bc5ba8c6754aa13b1127a7d2d79d580e53cc5b802aa5c637b193aa001181235` |

The expected model digest comes from the publisher's Hugging Face LFS metadata at the pinned revision. The backend archive digest comes from the pinned GitHub release. A digest proves correspondence, not safety or publisher identity by itself.

## Host and runtime

- MSI MS-7A78, Windows 10 Home 10.0.19045, x86_64.
- Intel Core i7-7700K at 4.20 GHz, four physical cores and eight logical processors.
- 15.96 GiB RAM; an NVIDIA GTX 1080 was present but not used.
- CPU-only execution, four threads, 2,048-token context, `gpu_layers = 0`.
- llama.cpp argument `--jinja`; seed 42, temperature 0, maximum 64 output tokens.
- Power state, thermal state, and unrelated system load were not measured or controlled.

See `results/environment.json` for the machine-readable record.

## Aggregate results

| Metric | Observed value | Scope |
|---|---:|---|
| Independent cold starts | 3 | One per benchmark invocation |
| Warm-ups | 2 per cold start | Not included as measured runs |
| Measured warm requests | 15 | Five per invocation |
| Failed measured requests | 0 | No failure exclusion was needed |
| Cold start, median | 835.371 ms | Range 660.720–977.129 ms |
| TTFT, median | 18.066 ms | Range 14.574–28.004 ms |
| Decode throughput, median | 54.394 tokens/s | Range 49.028–68.138 tokens/s |
| Decode sample standard deviation | 6.617 tokens/s | Descriptive only; not a confidence interval |

These values apply only to this host, artifact, backend, and runtime. They must not be used to claim superiority over another engine or hardware configuration.

Repeated prompts within one backend process may benefit from caching. TTFT includes HTTP and queueing. Observed prefill throughput is not a cold kernel measurement. No proxy-overhead, energy, memory, output-quality, safety, or thermal comparison is included.

## Evidence index

- `model.toml`: manifest used for all runs, with immutable source revision and pinned digests.
- `results/environment.json`: host, backend, model, and runtime metadata.
- `results/doctor.json`: successful real-prerequisite diagnostics.
- `results/verification.json`: successful model and backend integrity checks.
- `results/health.json` and `models.json`: real server health and model listing.
- `results/chat-nonstream.json`: real non-streaming chat response and backend timing fields.
- `results/chat-stream.sse`: real SSE deltas, usage, finish reason, and `[DONE]`.
- `results/conformance.json`: all three supported checks passed.
- `results/serve.log`: real startup timestamps; no prompts or credentials.
- `results/benchmark-01.json`, `benchmark-02.json`, `benchmark-03.json`: unmodified Caleta benchmark reports.
- `results/benchmark-01.log`, `benchmark-02.log`, `benchmark-03.log`: command outcomes.
- `results/summary.json`: aggregate derived from the three raw benchmark reports.
- `commands.txt`: reproduction commands with the validation API key redacted.

The non-streaming response is retained as protocol evidence, not as an exact-instruction-following or quality result. Prompts are synthetic and non-sensitive. Verification and diagnostic JSON retain local absolute paths as raw evidence; they contain no credentials.

## Reproduce

Install Rust 1.88 or newer and Python 3 if you use the optional mock test. From the repository root, build Caleta:

```bash
cargo build --release --locked
```

Download the exact backend archive and model into this experiment directory, preserving the relative paths in `model.toml`:

```bash
mkdir -p backend/bin model results

curl -fL \
  https://github.com/ggml-org/llama.cpp/releases/download/b11981/llama-b11981-bin-win-cpu-x64.zip \
  -o backend/llama-b11981-bin-win-cpu-x64.zip
unzip backend/llama-b11981-bin-win-cpu-x64.zip -d backend/bin

curl -fL \
  https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/9217f5db79a29953eb74d5343926648285ec7e67/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  -o model/qwen2.5-0.5b-instruct-q4_k_m.gguf
```

On Windows, run these commands in Git Bash. On another operating system, use the corresponding llama.cpp release asset, update `backend.executable` and its digest, and treat the result as a different environment—not a direct reproduction of this Windows binary.

Check the downloaded hashes before execution:

```bash
sha256sum backend/llama-b11981-bin-win-cpu-x64.zip
sha256sum backend/bin/llama-server.exe
sha256sum model/qwen2.5-0.5b-instruct-q4_k_m.gguf
```

Expected values are listed above and in `model.toml`. Do not run the artifacts if they differ; investigate the source rather than replacing the expected value with an unverified download.

Run diagnostics and verification:

```bash
target/release/caleta doctor --manifest model.toml --json
target/release/caleta verify model.toml --json
```

Start the service on loopback with a validation-only key:

```bash
export CALETA_API_KEY='replace-with-a-local-validation-secret'
target/release/caleta serve model.toml \
  --listen 127.0.0.1:18090 --record-out results/execution.json
```

Use another terminal for the requests in `commands.txt`, then stop the service before benchmarking. On Windows, forced termination does not produce the graceful-shutdown execution record used by the Unix tests.

Run three independent benchmarks:

```bash
for run in 01 02 03; do
  target/release/caleta bench model.toml \
    --output "results/benchmark-$run.json" \
    --runs 5 --warmup 2 \
    --prompt 'Explain local AI privacy in one concise paragraph.' \
    --max-tokens 64 --seed 42
 done
```

Do not compare new timings with the values above unless the host, artifacts, runtime, and methodology are equivalent. Publish all raw runs and any failed commands when reporting a replication.
