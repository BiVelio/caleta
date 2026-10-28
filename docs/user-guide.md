# Caleta user guide

This guide takes a new user from source checkout to a verified local model, an OpenAI-compatible request, and a reproducible report. Caleta v0.1 is experimental. Its automated end-to-end test uses a mock backend; real GGUF and hardware compatibility must be checked in each deployment.

Shell examples use bash syntax unless labeled otherwise. On Windows, run them in Git Bash or adapt them to PowerShell; PowerShell-specific commands are provided for paths, hashes, and environment variables.

## 1. What you need

- Rust 1.88 or newer, including Cargo. Install it through [rustup](https://rustup.rs/) if necessary.
- Git to clone the source.
- A current `llama-server` executable from [llama.cpp](https://github.com/ggml-org/llama.cpp). It must support `--offline`, `--no-webui`, and the `LLAMA_API_KEY` environment variable.
- A local GGUF model, permission to use it, and its actual license.
- Python 3 only for development and the deterministic mock test; it is not required to serve a real model.

Caleta does not download models, install llama.cpp, or provide an inference engine. Model artifacts and llama.cpp retain their own licenses.

Check the tools:

```bash
rustc --version
cargo --version
llama-server --version
```

If `llama-server` is not on your `PATH`, use its explicit path in `--backend`, `backend.executable`, and the examples below.

## 2. Build and install from source

```bash
git clone https://github.com/BiVelio/caleta.git
cd caleta
cargo build --release --locked
cargo install --path . --locked
```

`cargo install` places `caleta` in Cargo's binary directory. Make sure that directory is on your `PATH`. Alternatively, run the binary directly:

- Linux/macOS: `target/release/caleta`
- Windows PowerShell: `target\release\caleta.exe`

Run the initial diagnostic:

```bash
caleta doctor --backend /path/to/llama-server
caleta --help
```

On Windows PowerShell, an explicit backend path can look like `C:\tools\llama\llama-server.exe`. In TOML manifests, use forward slashes such as `C:/tools/llama/llama-server.exe` or escape every backslash.

## 3. Prepare a model directory

Put the manifest and GGUF in the same directory unless you deliberately use another relative or absolute artifact path:

```text
models/example/
├── model.toml
└── model.gguf
```

Calculate the model digest and exact byte length.

Linux:

```bash
sha256sum model.gguf
stat -c %s model.gguf
```

macOS:

```bash
shasum -a 256 model.gguf
stat -f %z model.gguf
```

Windows PowerShell:

```powershell
(Get-FileHash .\model.gguf -Algorithm SHA256).Hash.ToLower()
(Get-Item .\model.gguf).Length
```

Obtain the expected digest from a trusted publisher when possible. A digest computed after an untrusted download only creates a baseline for detecting later changes; it does not establish authenticity or safety.

## 4. Create the manifest

Copy the following into `model.toml` and replace every example value:

```toml
schema = "caleta.dev/model/v1"
id = "publisher/model:q4"
license = "REPLACE-WITH-THE-ACTUAL-MODEL-LICENSE"
source = "https://huggingface.co/publisher/model/resolve/main/model.gguf"

[artifact]
path = "model.gguf"
sha256 = "replace-with-the-actual-64-character-lowercase-sha256"
bytes = 123456789

[backend]
kind = "llama.cpp"
executable = "llama-server"

[runtime]
context_size = 4096
gpu_layers = -1
# threads = 8

[metadata]
quantization = "Q4_K_M"
revision = "publisher-commit-or-immutable-revision"
```

Use the artifact's real byte count and digest. Relative artifact paths resolve from the manifest directory, not from the directory where the command happens to run. `gpu_layers = -1` delegates GPU offload to llama.cpp; use `0` for CPU-only execution.

To pin the backend too, set an explicit executable path and its SHA-256:

```toml
[backend]
kind = "llama.cpp"
executable = "C:/tools/llama/llama-server.exe"
sha256 = "replace-with-the-backend-executable-sha256"
```

A backend digest cannot be combined with a bare executable name resolved through `PATH`. See [manifest.md](manifest.md) for all fields, defaults, managed arguments, and validation semantics.

## 5. Diagnose and verify

```bash
caleta doctor --manifest model.toml
caleta verify model.toml
caleta verify model.toml --json
```

`verify` checks the manifest, exact model size, model SHA-256, and the backend digest when pinned. A missing file, read error, size mismatch, or digest mismatch prevents `serve` and `bench`.

A successful result means **integrity only**. It does not prove publisher identity, model quality, license accuracy, or safe parsing.

## 6. Start the local API

```bash
caleta serve model.toml --record-out execution.json
```

The public listener defaults to `http://127.0.0.1:8080`. The supervised backend uses a separate random internal credential, binds to `127.0.0.1` on an ephemeral port, and receives `--offline` and `--no-webui`.

Leave the service running in this terminal. On Unix, stop it with Ctrl-C to perform graceful shutdown and write the execution record. Do not put secrets in the manifest.

To require a public API key even on loopback, set it before starting:

Linux/macOS:

```bash
export CALETA_API_KEY='replace-with-a-long-random-secret'
caleta serve model.toml --record-out execution.json
```

Windows PowerShell:

```powershell
$env:CALETA_API_KEY = 'replace-with-a-long-random-secret'
caleta serve model.toml --record-out execution.json
```

## 7. Make the first requests

List models:

```bash
curl http://127.0.0.1:8080/v1/models
```

If `CALETA_API_KEY` is configured:

```bash
curl http://127.0.0.1:8080/v1/models \
  -H "Authorization: Bearer $CALETA_API_KEY"
```

In Windows PowerShell, use `$env:CALETA_API_KEY` in place of `$CALETA_API_KEY`.

Send a streaming chat request, replacing the model identifier with the `id` in your manifest:

```bash
curl http://127.0.0.1:8080/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $CALETA_API_KEY" \
  -d '{
    "model": "publisher/model:q4",
    "messages": [{"role":"user","content":"Explain local AI in one sentence."}],
    "stream": true
  }'
```

Omit the Authorization header if no public key is configured. The supported surface is deliberately small: `/health`, `/v1/models`, and `/v1/chat/completions`. It is not the complete OpenAI API.

## 8. Check conformance

With the service running:

```bash
caleta conformance \
  --url http://127.0.0.1:8080 \
  --model publisher/model:q4 \
  --output conformance.json
```

The command reads the public key from `CALETA_API_KEY` when configured. It checks model listing, non-streaming chat, and basic SSE streaming. Passing does not certify tool calls, structured output, multimodality, cancellation, backpressure, fragmented SSE, or full OpenAI compatibility.

## 9. Run a reproducible benchmark

Stop other intensive workloads and run:

```bash
mkdir -p bench-results
caleta bench model.toml \
  --output bench-results/baseline.json \
  --runs 5 \
  --warmup 1 \
  --prompt 'Explain local AI privacy in one concise paragraph.' \
  --max-tokens 64 \
  --seed 42
```

The benchmark talks directly to the supervised backend; it does not measure public-proxy overhead. It performs one cold start, warm-ups, and measured warm requests in the same backend process. The JSON includes model identity and digest, backend version when available, OS/architecture, runtime parameters, cold start, raw successful and failed measured runs, and limitations.

By default, the prompt is represented by SHA-256 and byte length. Use `--include-prompt` only when storing its text is acceptable. Responses are not included. Warm-up failures are logged but not retained as raw per-run records.

See [benchmarking.md](benchmarking.md) for exact metric definitions. Mock timings are not meaningful LLM performance results. For engine comparisons, publish raw JSON and hold weights, quantization, prompts, decoding options, backend revision, hardware, and power conditions constant.

## 10. Deliberate LAN or remote exposure

Keep Caleta on loopback unless remote access is necessary. For a trusted LAN:

```bash
export CALETA_API_KEY='replace-with-a-long-random-secret'
caleta serve model.toml --listen 0.0.0.0:8080
```

A non-loopback listener refuses to start without a key of at least 16 characters. Use a high-entropy secret, firewall restrictions, and a TLS reverse proxy or SSH tunnel when traffic leaves the machine. Caleta itself serves plain HTTP. **Do not expose it directly to the public Internet.**

Browser origins must be allowed explicitly and repeatedly:

```bash
caleta serve model.toml \
  --cors-origin https://app.example \
  --cors-origin https://admin.example
```

Wildcard CORS is rejected. CORS does not replace authentication and does not protect non-browser clients.

## 11. Troubleshooting

| Symptom | What to check |
|---|---|
| `llama-server` not found | Install llama.cpp, use an explicit backend path, and run `caleta doctor --backend PATH`. |
| Manifest TOML parse error on Windows | Use forward slashes in TOML paths or escape backslashes. |
| Artifact size mismatch | Record the exact byte length of the intended GGUF, not an approximate download size. |
| Artifact digest mismatch | Do not bypass the error. Confirm the expected publisher digest and replace or reacquire the artifact through a trusted channel. |
| Backend digest requires explicit path | Replace `llama-server` with its full or manifest-relative path, or deliberately remove backend pinning. |
| Backend does not become ready | Run `llama-server --version`; confirm support for the required flags; inspect model resources and increase `--startup-timeout-secs` only if initialization genuinely needs more time. |
| API returns 401 | Set `CALETA_API_KEY` on the client and server, and send `Authorization: Bearer ...`. |
| Non-loopback listener refuses to start | Set a public key of at least 16 characters; preferably use a longer random secret. |
| Benchmark has failed runs | Inspect the raw `runs` entries and backend version. Do not delete failures before publishing aggregate results. |
| Real model behaves differently from the mock | The mock validates orchestration only. Test the exact GGUF and llama.cpp revision and report versions, hardware, and configuration. |

For diagnostics that can be attached to an issue:

```bash
caleta --version
caleta doctor --manifest model.toml --json
caleta verify model.toml --json
```

Remove credentials and sensitive paths before sharing reports. Do not upload proprietary model files or sensitive prompts. Security reports follow the private process in [SECURITY.md](../SECURITY.md), not public issues.

## 12. Security and privacy boundary

Caleta adds no telemetry or prompt history and does not log request or response bodies. It requests offline backend behavior and limits backend listening to loopback. These controls do not isolate the host OS, client applications, backend vulnerabilities, swap, crash reports, or privileged local processes.

`--offline` is not an OS-enforced network sandbox. Execution JSON is unsigned operational evidence, not cryptographic attestation. Protect model and backend directories from untrusted writers and review [threat-model.md](threat-model.md) before a high-assurance deployment.

## 13. Development verification

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
CALETA_BIN=target/debug/caleta python3 tests/e2e_mock.py
```

The mock E2E does not test real inference, GGUF parsing, GPU compatibility, quality, or performance. Unix also covers graceful shutdown and execution records; Windows uses forced process-tree shutdown for the mock. Contribution requirements are in [CONTRIBUTING.md](../CONTRIBUTING.md).
