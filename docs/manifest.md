# Model manifest `caleta.dev/model/v1`

The manifest is a declarative execution contract. It is intentionally TOML so that a person can review it before running a model. Unknown TOML fields are currently ignored by Serde; producers should not rely on them. `metadata` is the extension point for quantization, immutable source revision, publisher, and other descriptive values.

## Required fields

| Field | Type | Meaning |
|---|---|---|
| `schema` | string | Must be exactly `caleta.dev/model/v1` |
| `id` | string | Model identifier and upstream alias; 1–128 ASCII letters, digits, `.`, `_`, `-`, `/`, `:` |
| `license` | string | Actual model license or an accurate license identifier; never inferred by Caleta |
| `source` | URL | `https://` or `file://` origin declared by the manifest author |
| `artifact.path` | path | GGUF file, absolute or relative to the manifest |
| `artifact.sha256` | 64 hex characters | Expected artifact digest |
| `artifact.bytes` | positive integer | Exact expected file size |
| `backend.kind` | string | `llama.cpp` in v0.1 |
| `backend.executable` | path or name | `llama-server` by default; explicit path required for digest pinning |

`description`, `backend.sha256`, `backend.args`, `runtime`, and `metadata` are optional. Runtime defaults are 4,096 context tokens and `-1` GPU layers, delegating device offload to llama.cpp defaults. `threads` is unset unless declared.

## Backend arguments

Advanced `backend.args` are passed to `llama-server` after Caleta-managed arguments. Caleta rejects flags that could replace the model, listener, credentials, runtime values, TLS boundary, or enable agent/tools and Web UI functionality. This denylist is defense in depth, not a parser for every current or future llama.cpp option. Review any additional flags against the exact backend version.

Inherited environment variables beginning with `LLAMA_ARG_`, plus `LLAMA_API_KEY`, are removed from the child environment. Use explicit manifest values for supported options. Other environment variables remain available because GPU drivers and dynamic loaders may need them.

## Example with backend pinning

```toml
schema = "caleta.dev/model/v1"
id = "acme/example:q4"
license = "Apache-2.0"
source = "https://models.example/acme-example-q4.gguf"

[artifact]
path = "acme-example-q4.gguf"
sha256 = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
bytes = 104857600

[backend]
kind = "llama.cpp"
executable = "./bin/llama-server"
sha256 = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
args = ["--jinja"]

[runtime]
context_size = 8192
gpu_layers = -1
threads = 8

[metadata]
quantization = "Q4_K_M"
revision = "immutable-source-commit-or-revision"
```

## Verification semantics

The verifier reads the file in 1 MiB chunks, so artifact size does not determine verifier memory use. Size mismatch takes precedence over hash mismatch in the reported status. Missing files, read errors, size mismatches, and digest mismatches all prevent serving or benchmarking. A backend digest can be checked only for an explicit path; `PATH` lookup is rejected when `backend.sha256` is present because the resolved binary would otherwise be ambiguous.

Verification and process execution are separate operations. Caleta does not hold an open, verified file descriptor for llama.cpp, so a local actor able to replace files between verification and backend loading can create a time-of-check/time-of-use race. Protect model and backend directories from untrusted writers. Future versions can reduce this gap with immutable artifact stores and OS-level mechanisms.
