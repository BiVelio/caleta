//! Caleta is a trust layer for local model runtimes.
//!
//! It deliberately does not implement inference kernels. It verifies model
//! artifacts, supervises an existing runtime, and exposes a small, safer
//! OpenAI-compatible surface.

pub mod audit;
pub mod backend;
pub mod bench;
pub mod conformance;
pub mod doctor;
pub mod manifest;
pub mod server;
pub mod verify;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MODEL_SCHEMA: &str = "caleta.dev/model/v1";
pub const EXECUTION_SCHEMA: &str = "caleta.dev/execution/v1";
pub const BENCHMARK_SCHEMA: &str = "caleta.dev/benchmark/v1";
pub const CONFORMANCE_SCHEMA: &str = "caleta.dev/conformance/v1";
