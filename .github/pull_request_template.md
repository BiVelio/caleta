## Problem and change

<!-- What changes, for whom, and which alternatives were considered? -->

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --locked --all-targets -- -D warnings`
- [ ] `cargo test --locked`
- [ ] Mock E2E, or an explanation of why it does not apply
- [ ] Real-model validation, when backend/inference behavior changes

## Boundaries

- [ ] No credentials, private prompts, or unlicensed model artifacts are included.
- [ ] Security/privacy documentation and threat model are updated where needed.
- [ ] Claims distinguish integrity, authenticity, offline flags, and OS enforcement.
- [ ] Benchmark changes preserve raw failures and state metric limitations.
