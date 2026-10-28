# Architecture

## Decision

Caleta v0.1 is a process supervisor and trust boundary around `llama-server`, not a library linked into an inference engine and not a new kernel implementation. This keeps the core small, preserves GGUF interoperability, inherits platform support from llama.cpp, and allows the manifest and report formats to be reused by other runtimes.

The public proxy and benchmark client use a separate internal credential on every backend launch. The backend binds only to IPv4 loopback. The public listener defaults to loopback and requires a key for any non-loopback address. Prompts and responses pass through memory but are not logged or persisted by Caleta. Request bodies are buffered with a 16 MiB limit; response bodies, including SSE, are streamed without collection.

## Components

| Component | Responsibility | Explicit non-responsibility |
|---|---|---|
| Manifest loader | Validate schema, identifiers, declared provenance, runtime values, and forbidden backend flags | Establish that source or license declarations are truthful |
| Verifier | Stream artifact contents through SHA-256 and compare exact size and digest | Scan model content, verify a signature, or establish publisher identity |
| Backend supervisor | Remove inherited `LLAMA_ARG_*`, create an internal key, start `llama-server`, wait for health, and kill it on shutdown | Sandbox the process at OS level or defend against a malicious backend |
| HTTP proxy | Authenticate external requests, expose two OpenAI-compatible routes, filter headers, stream responses | Provide full OpenAI API compatibility or TLS |
| Execution record | Capture prompt-free configuration and verification evidence | Produce signed attestation or prove what happened after startup |
| Benchmark | Preserve cold-start and warm-run metrics, environment, usage, and failures | Rank engines universally or measure output quality and energy |
| Conformance | Check a small API contract against Caleta or another server | Certify complete OpenAI compatibility |

## Process and data flow

1. The CLI parses and validates the TOML manifest. Paths for artifacts and explicit relative backend paths resolve against the manifest directory; a bare backend name resolves through `PATH`.
2. `serve` and `bench` refuse to continue if the model artifact does not match. When an explicit backend digest is supplied, the backend must match too.
3. The supervisor reserves an ephemeral loopback port, starts `llama-server` with `--offline`, `--no-webui`, a model alias, runtime options, and `LLAMA_API_KEY`. Inherited `LLAMA_ARG_*` and `LLAMA_API_KEY` variables are removed first.
4. Readiness is determined by an authenticated `/health` request. Startup timeout and premature process exit are errors.
5. The proxy accepts only its declared routes. If configured, public bearer credentials are compared in constant time. The upstream `Authorization` header is always replaced with the internal key.
6. Hop-by-hop and upstream CORS response headers are removed. CORS headers are generated only from Caleta's explicit origin allowlist.
7. On Ctrl-C or fatal server exit, Caleta terminates and waits for the child. Tokio's kill-on-drop behavior provides an additional process-lifetime backstop.

## Trust and authenticity

There are three different statements that should not be conflated:

- **Integrity:** the bytes read match the digest in the manifest.
- **Provenance declaration:** the manifest states where the artifact came from and which license it declares.
- **Authenticated provenance:** a trusted mechanism establishes who supplied the manifest and digest.

Caleta v0.1 implements the first two. The third requires a trusted distribution channel, signed manifests, transparency, or attestations. Adding a self-generated hash after an untrusted download does not provide it.

## Extension points

Future backends should implement the same readiness, internal authentication, offline, and shutdown contract rather than bypassing it. Uzu, MLX, or other engines can also adopt the JSON schemas without adopting the supervisor. Signed manifests, OS-specific network isolation, energy instrumentation, and quality evaluation should be separate, testable additions rather than changes to the definition of “verified.”
