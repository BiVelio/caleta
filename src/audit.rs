use crate::manifest::LoadedManifest;
use crate::verify::VerificationReport;
use crate::{EXECUTION_SCHEMA, VERSION};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

/// Prompt-free evidence about one supervised execution. This record is not a
/// signature and does not prove network isolation; it makes a run inspectable.
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionRecord {
    pub schema: String,
    pub created_at: DateTime<Utc>,
    pub caleta_version: String,
    pub model_id: String,
    pub manifest_path: PathBuf,
    pub model_sha256: String,
    pub backend_kind: String,
    pub backend_path: PathBuf,
    pub backend_sha256: Option<String>,
    pub backend_dependencies: Vec<String>,
    pub backend_version: Option<String>,
    pub listen: SocketAddr,
    pub backend_loopback_only: bool,
    pub backend_offline_flag: bool,
    pub external_authentication: bool,
    pub cors_origins: Vec<String>,
    pub context_size: u32,
    pub gpu_layers: i32,
    pub threads: Option<u32>,
    pub extra_backend_args: Vec<String>,
    pub model_integrity_verified: bool,
    pub os: String,
    pub arch: String,
    pub prompt_content_recorded: bool,
    pub limitations: Vec<String>,
}

impl ExecutionRecord {
    pub fn new(
        loaded: &LoadedManifest,
        verification: &VerificationReport,
        listen: SocketAddr,
        external_authentication: bool,
        cors_origins: Vec<String>,
        backend_version: Option<String>,
    ) -> Self {
        Self {
            schema: EXECUTION_SCHEMA.into(),
            created_at: Utc::now(),
            caleta_version: VERSION.into(),
            model_id: loaded.manifest.id.clone(),
            manifest_path: loaded.manifest_path.clone(),
            model_sha256: loaded.manifest.artifact.sha256.to_ascii_lowercase(),
            backend_kind: loaded.manifest.backend.kind.clone(),
            backend_path: loaded.backend_path(),
            backend_sha256: loaded.manifest.backend.sha256.clone(),
            backend_dependencies: loaded
                .manifest
                .backend
                .dependencies
                .iter()
                .map(|dependency| format!("{}:{}", dependency.path.display(), dependency.sha256))
                .collect(),
            backend_version,
            listen,
            backend_loopback_only: true,
            backend_offline_flag: true,
            external_authentication,
            cors_origins,
            context_size: loaded.manifest.runtime.context_size,
            gpu_layers: loaded.manifest.runtime.gpu_layers,
            threads: loaded.manifest.runtime.threads,
            extra_backend_args: loaded.manifest.backend.args.clone(),
            model_integrity_verified: verification.verified,
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            prompt_content_recorded: false,
            limitations: vec![
                "integrity_does_not_establish_publisher_authenticity".into(),
                "offline_flag_is_not_os_network_sandboxing".into(),
                "record_is_unsigned_and_does_not_prove_execution".into(),
            ],
        }
    }

    pub async fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        let data = serde_json::to_vec_pretty(self)?;
        tokio::fs::write(path, data)
            .await
            .with_context(|| format!("cannot write execution record {}", path.display()))?;
        Ok(())
    }
}
