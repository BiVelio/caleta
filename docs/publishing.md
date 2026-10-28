# Publishing and release runbook

## Before the first public push

1. Confirm that BiVelio approves the name **Caleta**, tagline **“Private local AI, with evidence” / “Tu IA, en puerto seguro”**, and Apache-2.0 licensing.
2. Check the exact name on GitHub, crates.io, package registries, relevant domains, and trademark registers. The initial web search found no prominent local-inference project with this name, but that is not legal clearance.
3. Create an empty repository under the confirmed BiVelio namespace. Do not add a second license or overwrite this repository's community files.
4. Enable GitHub private vulnerability reporting, branch protection, required CI checks, and artifact attestations when the release process is ready.
5. Replace no source files with generated or AI-attributed authorship claims. Review the complete diff and ensure the copyright holder in `LICENSE` is correct.

```bash
git init
git add .
git commit -m "Initial Caleta trust-layer release"
git branch -M main
git remote add origin git@github.com:BiVelio/caleta.git
git push -u origin main
```

A prepared source archive is not itself a public release. The person performing these steps needs permission to create and push to the destination repository; no credentials are included in this project.

## Release checklist

- `cargo fmt --all -- --check`
- `cargo clippy --locked --all-targets -- -D warnings`
- `cargo test --locked`
- `CALETA_BIN=target/debug/caleta python3 tests/e2e_mock.py`
- Test at least one real GGUF model with a pinned llama.cpp build on each claimed platform.
- Review `Cargo.lock`, release binaries, and generated SHA-256 sums.
- Update `CHANGELOG.md`, version fields, documentation, and roadmap status without changing the meaning of prior reports.
- Create a signed, annotated tag such as `v0.1.0`, then create the GitHub release. The release workflow builds when that release is published.
- Verify uploaded archives and sums on a clean machine. Add GitHub artifact provenance and verification instructions before claiming authenticated releases.

## Publishing strategy

Lead with the problem and evidence, not a speed claim: “Caleta makes local inference inspectable: verified artifacts, safer defaults, prompt-free execution records, and reproducible benchmarks for llama.cpp.” Publish the technical launch article on [BiVelio Research](https://research.bivelio.com) and link it from [BiVelio](https://bivelio.com) as an open infrastructure project. Include one real-model demo, the threat model, an example report, and a comparison that acknowledges existing engines. Explain that Caleta complements BiVelio's governed/privacy vision but is not Studio, Privacy, or the Trust Layer and has no claimed product integration in v0.1. Invite backend maintainers to reuse the manifest and conformance/benchmark schemas without adopting Caleta's supervisor.

Good first issues should include signed manifests, real-model test fixtures under appropriately licensed redistribution terms, OS-specific network isolation, GPU memory instrumentation, energy measurement, and conformance coverage for fragmented SSE. Do not label an issue “good first issue” if it requires changing the security boundary without a design review.
