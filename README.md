# Caleta

**Private local AI, with evidence.**

An open-source infrastructure project from [BiVelio Research](https://research.bivelio.com), aligned with [BiVelio](https://bivelio.com)'s work on governed, traceable AI operations.

Caleta is an open-source trust layer for local AI inference. It verifies model artifacts, supervises `llama.cpp`, exposes a small OpenAI-compatible API, and produces reproducible benchmark and conformance reports.

It is **not another inference engine**. Caleta does not implement kernels and does not claim to be faster than Uzu, llama.cpp, MLX, Ollama, or mistral.rs. It makes an existing local runtime easier to inspect, reproduce, and expose safely.

> **Project status:** v0.1.0 experimental. Local software validation passes, including 15 unit tests and deterministic end-to-end checks. Real GGUF and hardware validation is still required before treating a deployment as validated. The crates.io name, domains, and trademarks remain subject to owner-side availability checks.

**New here?** Follow the [complete user guide](docs/user-guide.md). The quick start below is a shorter path for users already familiar with Rust, llama.cpp, and GGUF.

**Research:** read the [technical preprint](paper/caleta.pdf) or its [LaTeX source](paper/caleta.tex), authored by Joan-Marc Fisa (BiVelio Inc). The manuscript is not peer reviewed and distinguishes mock-based software validation from real-model evidence.

## Why Caleta?

Caleta extends BiVelio Research's commitment to open reasoning, code, and real measurements into local inference. It can complement a self-hosted privacy architecture by making model execution inspectable. **It is not BiVelio Studio, BiVelio Privacy, or the BiVelio Trust Layer, and no integration with those products is claimed in v0.1.** The repository stands on its own and can be used without any BiVelio product.

Local inference is often described as “private” without distinguishing several different claims: where prompts go, what the backend can access, which artifact was executed, what was measured, and which interfaces are exposed. Caleta turns part of that ambiguity into explicit configuration and prompt-free evidence.

- **GGUF by default:** use an interoperable model artifact instead of creating another model format.
- **Integrity before execution:** stream SHA-256 verification and check exact file size.
- **Declared provenance:** every manifest names its source and license. Caleta does not pretend that a hash alone establishes publisher authenticity.
- **Loopback backend:** `llama-server` is bound to `127.0.0.1`, launched with `--offline`, and given a random internal API key through its environment.
- **Safe public defaults:** the proxy also starts on loopback. Non-loopback exposure requires an API key of at least 16 characters. CORS is disabled unless explicit origins are supplied.
- **Small API surface:** only `/health`, `/v1/models`, and `/v1/chat/completions`; no agent tools, prompt history, or telemetry.
- **Evidence, not slogans:** optional execution records and JSON benchmark/conformance reports preserve configuration, failures, and limitations.

## Quick start

### Prerequisites

- Rust 1.88 or newer.
- A current `llama-server` from [llama.cpp](https://github.com/ggml-org/llama.cpp), including support for `--offline`, `--no-webui`, and `LLAMA_API_KEY`.
- A local GGUF model and its actual license.

```bash
git clone https://github.com/BiVelio/caleta.git
cd caleta
cargo build --release --locked
caleta doctor --backend /path/to/llama-server
```

### 1. Create a model manifest

Copy `examples/model.toml` next to your GGUF file and update the source, license, size, and digest:

```bash
sha256sum model.gguf       # macOS: shasum -a 256 model.gguf
stat -c %s model.gguf      # macOS: stat -f %z model.gguf
```

```toml
schema = "caleta.dev/model/v1"
id = "publisher/model:q4"
license = "Apache-2.0" # example only: use the model's actual license
source = "https://huggingface.co/publisher/model/resolve/main/model.gguf"

[artifact]
path = "model.gguf"
sha256 = "replace-with-64-lowercase-hex-characters"
bytes = 123456789

[backend]
kind = "llama.cpp"
executable = "llama-server"

[runtime]
context_size = 4096
gpu_layers = -1
# threads = 8
```

Obtain the digest from the publisher when possible. Computing a digest after downloading an untrusted file only establishes a baseline for future comparisons; it does not make the file authentic or safe.

### 2. Verify and serve

```bash
caleta verify examples/model.toml
caleta serve examples/model.toml --record-out execution.json
```

The API listens on `http://127.0.0.1:8080` by default:

```bash
curl http://127.0.0.1:8080/v1/models

curl http://127.0.0.1:8080/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "publisher/model:q4",
    "messages": [{"role":"user","content":"Explain local AI in one sentence."}],
    "stream": true
  }'
```

### 3. Expose deliberately

For LAN access, set a secret in the environment and use an explicit bind address:

```bash
export CALETA_API_KEY='replace-with-a-long-random-secret'
caleta serve model.toml --listen 0.0.0.0:8080
curl http://HOST:8080/v1/models -H "Authorization: Bearer $CALETA_API_KEY"
```

Caleta serves plain HTTP. Use a TLS reverse proxy, SSH tunnel, or an appropriately trusted private network when traffic leaves the machine. Do not expose the service directly to the public internet. Browser origins must be listed explicitly with repeated `--cors-origin https://app.example` flags; wildcard CORS is rejected.

## Commands

| Command | Purpose | Output |
|---|---|---|
| `caleta verify MODEL.toml [--json]` | Check artifact size and SHA-256; check backend digest when pinned | Human-readable or `caleta.dev/verification/v1` JSON |
| `caleta serve MODEL.toml` (`run` alias) | Verify, start a supervised backend, and proxy the supported API | Prompt-free logs; optional execution JSON |
| `caleta doctor [--manifest MODEL.toml] [--backend PATH]` | Check manifest, artifact, and backend prerequisites | Human-readable or JSON diagnostics |
| `caleta bench MODEL.toml --output RESULT.json` | Measure one cold start and repeated warm requests | `caleta.dev/benchmark/v1` JSON, including failures |
| `caleta conformance --url URL [--output RESULT.json]` | Test models, non-streaming chat, and SSE against a server | `caleta.dev/conformance/v1` JSON |

The benchmark reports TTFT, observed prefill throughput, decode throughput, content-event inter-arrival p50/p95/p99, token usage, cold start, and main-process peak RSS on Linux. It does **not** claim to measure GPU memory, energy, or output quality. Prompt text is hashed rather than stored unless `--include-prompt` is explicitly supplied.

## Architecture

```text
OpenAI-compatible client
          │ public API key (optional on loopback, required otherwise)
          ▼
      Caleta proxy ─────── /health, execution evidence, CORS policy
          │ random internal key; loopback only
          ▼
     llama-server ──────── --offline; no Web UI; no inherited LLAMA_ARG_*
          │
          ▼
   verified local GGUF artifact
```

See the [user guide](docs/user-guide.md) for installation and operation, and [architecture](docs/architecture.md), [manifest](docs/manifest.md), [threat model](docs/threat-model.md), [benchmarking](docs/benchmarking.md), and [publishing](docs/publishing.md) for contracts, limitations, development, and release procedures.

## What “private” means here

Caleta does not add telemetry or prompt persistence, and it asks the backend to operate offline. These are meaningful controls, but not an absolute guarantee. The host OS, backend, model runtime, native dependencies, other local processes, and client applications remain part of the trust boundary. `--offline` is not an OS-level network sandbox. A SHA-256 digest proves correspondence with an expected value, not publisher authenticity, model quality, license accuracy, or safety.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
CALETA_BIN=target/debug/caleta python3 tests/e2e_mock.py
```

The deterministic mock backend exercises verification, process supervision, authentication, models, streaming, conformance, audit records, and benchmarks without downloading a model. Unix tests also cover graceful shutdown and its execution record; Windows uses forced process-tree shutdown for the mock. It does not test real inference, GPU backends, performance, or llama.cpp compatibility.

## Contributing and security

Contributions are welcome; start with [`CONTRIBUTING.md`](CONTRIBUTING.md). Please do not disclose vulnerabilities in public issues; use the private process in [`SECURITY.md`](SECURITY.md). Participation is governed by the [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md).

## License

Apache-2.0. Model artifacts and llama.cpp retain their own licenses; Caleta does not relicense them. “Caleta” and BiVelio branding are not granted as trademarks by the software license.
