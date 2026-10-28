# Changelog

All notable changes follow Keep a Changelog conventions and semantic versioning.

## [0.1.0] - 2026-10-28

### Added

- Versioned TOML model manifests with declared source, license, artifact size, SHA-256, backend, and runtime options.
- Streaming artifact verification and optional backend-binary pinning.
- Supervised `llama-server` execution with loopback binding, offline mode, random internal authentication, sanitized inherited options, readiness checks, and shutdown.
- OpenAI-compatible models and chat-completions proxy with streaming SSE, bounded request bodies, constant-time public-key comparison, and explicit CORS allowlists.
- Prompt-free execution records and explicit privacy/authenticity limitations.
- Reproducible warm-run benchmarks with separate cold start, TTFT, throughput, inter-event latency, usage, Linux RSS, and preserved failures.
- Reusable OpenAI API conformance smoke checks.
- Unit tests and deterministic end-to-end mock coverage for verify, serve, authentication, SSE, conformance, and bench; graceful-shutdown execution records are covered on Unix.
- Cross-platform CI, release build workflow, security policy, contribution guide, governance, and release documentation.

### Known limitations

- No real-model or hardware performance claim is included; the automated E2E uses a mock backend.
- No signed manifests, OS-enforced network sandbox, TLS server, energy measurement, GPU memory accounting, or model-quality evaluation.
- Only two OpenAI-compatible inference routes are exposed; this is not full API compatibility.
