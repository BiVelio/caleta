use crate::MODEL_SCHEMA;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManifest {
    pub schema: String,
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    pub license: String,
    pub source: String,
    pub artifact: Artifact,
    pub backend: BackendSpec,
    #[serde(default)]
    pub runtime: RuntimeSpec,
    #[serde(default)]
    pub metadata: toml::Table,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub path: PathBuf,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendDependency {
    pub path: PathBuf,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendSpec {
    #[serde(default = "llama_backend")]
    pub kind: String,
    #[serde(default = "llama_executable")]
    pub executable: PathBuf,
    /// Optional digest for the backend executable. A name resolved through PATH
    /// cannot be pinned reliably and is therefore rejected when this is set.
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<BackendDependency>,
    /// Additional llama-server flags. Flags that conflict with Caleta's
    /// process/network boundary are rejected during validation.
    #[serde(default)]
    pub args: Vec<String>,
}

fn llama_backend() -> String {
    "llama.cpp".into()
}
fn llama_executable() -> PathBuf {
    "llama-server".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeSpec {
    #[serde(default = "default_context")]
    pub context_size: u32,
    #[serde(default = "default_gpu_layers")]
    pub gpu_layers: i32,
    #[serde(default)]
    pub threads: Option<u32>,
}

impl Default for RuntimeSpec {
    fn default() -> Self {
        Self {
            context_size: default_context(),
            gpu_layers: default_gpu_layers(),
            threads: None,
        }
    }
}

fn default_context() -> u32 {
    4096
}
fn default_gpu_layers() -> i32 {
    -1
}

#[derive(Debug, Clone)]
pub struct LoadedManifest {
    pub manifest: ModelManifest,
    pub manifest_path: PathBuf,
    pub base_dir: PathBuf,
}

impl LoadedManifest {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read manifest {}", path.display()))?;
        let manifest: ModelManifest = toml::from_str(&text)
            .with_context(|| format!("invalid TOML manifest {}", path.display()))?;
        manifest.validate()?;
        let manifest_path = std::fs::canonicalize(path)
            .with_context(|| format!("cannot resolve manifest {}", path.display()))?;
        let base_dir = manifest_path
            .parent()
            .context("manifest has no parent directory")?
            .to_path_buf();
        Ok(Self {
            manifest,
            manifest_path,
            base_dir,
        })
    }

    pub fn artifact_path(&self) -> PathBuf {
        if self.manifest.artifact.path.is_absolute() {
            self.manifest.artifact.path.clone()
        } else {
            self.base_dir.join(&self.manifest.artifact.path)
        }
    }

    pub fn backend_path(&self) -> PathBuf {
        let path = &self.manifest.backend.executable;
        if path.is_absolute() || is_program_name(path) {
            path.clone()
        } else {
            self.base_dir.join(path)
        }
    }
}

impl ModelManifest {
    pub fn validate(&self) -> Result<()> {
        if self.schema != MODEL_SCHEMA {
            bail!(
                "unsupported schema {:?}; expected {MODEL_SCHEMA}",
                self.schema
            );
        }
        if self.id.trim().is_empty() || self.id.len() > 128 {
            bail!("model id must contain 1–128 characters");
        }
        if !self
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/' | ':'))
        {
            bail!("model id contains unsupported characters");
        }
        if self.license.trim().is_empty() {
            bail!("license must be declared; use the model's actual license");
        }
        if !(self.source.starts_with("https://") || self.source.starts_with("file://")) {
            bail!("source must be an https:// or file:// URL");
        }
        validate_digest(&self.artifact.sha256)?;
        if self.artifact.bytes == 0 {
            bail!("artifact.bytes must be greater than zero");
        }
        if self.artifact.path.as_os_str().is_empty() {
            bail!("artifact.path cannot be empty");
        }
        if self.backend.kind != "llama.cpp" {
            bail!(
                "unsupported backend {:?}; v0.1 supports llama.cpp",
                self.backend.kind
            );
        }
        if self.backend.executable.as_os_str().is_empty() {
            bail!("backend.executable cannot be empty");
        }
        if let Some(digest) = &self.backend.sha256 {
            validate_digest(digest)?;
            if is_program_name(&self.backend.executable) {
                bail!("backend.sha256 requires an explicit executable path, not a PATH lookup");
            }
        }
        for dependency in &self.backend.dependencies {
            validate_digest(&dependency.sha256)?;
            if dependency.path.is_absolute()
                || dependency.path.components().any(|component| {
                    matches!(
                        component,
                        Component::ParentDir | Component::RootDir | Component::Prefix(_)
                    )
                })
            {
                bail!("backend dependency paths must stay inside the backend directory");
            }
        }
        if self.runtime.context_size == 0 {
            bail!("runtime.context_size must be greater than zero");
        }
        if self.runtime.threads == Some(0) {
            bail!("runtime.threads must be greater than zero when set");
        }
        validate_backend_args(&self.backend.args)?;
        Ok(())
    }
}

fn validate_digest(value: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("SHA-256 must contain exactly 64 hexadecimal characters");
    }
    Ok(())
}

fn is_program_name(path: &Path) -> bool {
    path.components().count() == 1 && matches!(path.components().next(), Some(Component::Normal(_)))
}

fn validate_backend_args(args: &[String]) -> Result<()> {
    let forbidden: BTreeSet<&'static str> = [
        "--model",
        "-m",
        "--host",
        "--port",
        "--api-key",
        "--api-key-file",
        "--alias",
        "--offline",
        "--no-offline",
        "--ctx-size",
        "-c",
        "--n-gpu-layers",
        "-ngl",
        "--threads",
        "-t",
        "--tools",
        "--agent",
        "--webui",
        "--no-webui",
        "--ssl-key-file",
        "--ssl-cert-file",
    ]
    .into_iter()
    .collect();

    for arg in args {
        let flag = arg.split('=').next().unwrap_or(arg);
        if forbidden.contains(flag) {
            bail!(
                "backend argument {flag:?} is managed by Caleta or outside its security boundary"
            );
        }
        if arg.contains('\0') || arg.contains('\n') || arg.contains('\r') {
            bail!("backend arguments cannot contain NUL or newline characters");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> ModelManifest {
        toml::from_str(&format!(
            r#"
schema = "{MODEL_SCHEMA}"
id = "org/model:q4"
license = "Apache-2.0"
source = "https://example.test/model.gguf"
[artifact]
path = "model.gguf"
sha256 = "{}"
bytes = 42
[backend]
kind = "llama.cpp"
executable = "llama-server"
"#,
            "a".repeat(64)
        ))
        .unwrap()
    }

    #[test]
    fn accepts_minimal_manifest() {
        valid().validate().unwrap();
    }

    #[test]
    fn rejects_short_digest() {
        let mut manifest = valid();
        manifest.artifact.sha256 = "abcd".into();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn rejects_network_override() {
        let mut manifest = valid();
        manifest.backend.args = vec!["--host=0.0.0.0".into()];
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn rejects_agent_tools() {
        let mut manifest = valid();
        manifest.backend.args = vec!["--tools".into()];
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn rejects_unpinned_backend_digest() {
        let mut manifest = valid();
        manifest.backend.sha256 = Some("b".repeat(64));
        assert!(manifest.validate().is_err());
    }
}
