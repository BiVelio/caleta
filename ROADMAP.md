# Roadmap

The roadmap prioritizes verifiable capabilities over feature count. Items are proposals until scheduled for a release.

## v0.1 — publishable trust-layer base

- [x] GGUF manifest and SHA-256 verification
- [x] llama.cpp process supervision and safer network defaults
- [x] OpenAI-compatible models/chat proxy with SSE
- [x] Execution evidence, benchmark JSON, and conformance smoke tests
- [x] Mock-based unit/E2E tests and publication documentation
- [ ] Owner-side name/trademark check
- [x] Initial pinned real-GGUF validation on Windows CPU

## v0.2 — stronger provenance and real-world validation

- Signed manifests and transparent verification-failure reasons
- Published schema documents and compatibility tests for other engines
- Real-model hardware matrix with immutable backend builds
- Fragmented SSE, backpressure, cancellation, and concurrent-request tests
- Optional SBOM and GitHub artifact attestations for releases

## v0.3 — platform enforcement and measurement

- OS-specific outbound-network denial with documented enforcement tests
- Immutable artifact store to reduce verification/execution races
- GPU and process-tree memory metrics where supported
- Calibrated energy and thermal measurements on supported hardware
- Output-quality regression suites for quantization comparisons

## Later, only with evidence of need

- Additional backends such as Uzu or MLX behind the same contract
- Multi-model routing and concurrent workload benchmarks
- Optional desktop UI without expanding the default network surface
- SDKs generated from a stable API contract

A graphical interface, a proprietary model format, or a claim of universal performance leadership is not a prerequisite for community value.
