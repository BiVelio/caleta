# Contributing

Thank you for improving Caleta. Contributions should make local inference easier to verify, reproduce, or operate safely without inflating the core with unsupported claims.

## Development setup

Install Rust 1.88 or newer, Python 3 for the mock E2E, and a current llama.cpp build for real-model testing. Clone the repository and run:

```bash
cargo fmt --all
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
CALETA_BIN=target/debug/caleta python3 tests/e2e_mock.py
```

Never commit real prompts, credentials, proprietary model artifacts, or large GGUF files. Tests should use deterministic fixtures. Changes that need a real model must describe the model source, license, quantization, hardware, backend revision, and exact command without assuming that CI can download it.

## Pull request expectations

- Keep changes focused and explain user-facing behavior.
- Add tests for success and failure paths, especially authentication, streaming, integrity, and process cleanup.
- Update the manifest schema and documentation together; preserve backward compatibility or introduce a new schema version.
- Update `docs/threat-model.md` for security-boundary changes.
- Do not claim performance superiority without comparable raw measurements.
- Do not call integrity “authenticity,” `--offline` “network sandboxing,” or a small conformance suite “full OpenAI compatibility.”
- Run formatting, Clippy, unit tests, and mock E2E before requesting review.

By contributing, you agree that your contribution is licensed under Apache-2.0. You must have the right to submit it. Do not include copied code without preserving its license and confirming compatibility.

## Reporting bugs

Use the bug-report template and include Caleta version, OS/architecture, llama.cpp version, manifest with secrets removed, command, expected/actual behavior, and prompt-free logs. Do not include sensitive prompts or model files. Use the private security process instead when the bug exposes credentials, prompts, artifacts, or an unsafe listener.
