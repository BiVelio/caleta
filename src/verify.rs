use crate::manifest::LoadedManifest;
use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntegrityStatus {
    Verified,
    Missing,
    SizeMismatch,
    HashMismatch,
    ReadError,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileVerification {
    pub path: PathBuf,
    pub expected_sha256: String,
    pub actual_sha256: Option<String>,
    pub expected_bytes: u64,
    pub actual_bytes: Option<u64>,
    pub status: IntegrityStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationReport {
    pub schema: String,
    pub model: String,
    pub artifact: FileVerification,
    pub backend: Option<FileVerification>,
    pub backend_pinned: bool,
    pub backend_dependencies: Vec<FileVerification>,
    pub backend_dependencies_verified: bool,
    pub verified: bool,
    /// SHA-256 verifies integrity against the declared digest. Authenticity
    /// still depends on how the manifest and digest were obtained.
    pub authenticity: String,
}

pub async fn verify_file(
    path: impl AsRef<Path>,
    expected_sha256: &str,
    expected_bytes: u64,
) -> FileVerification {
    let path = path.as_ref().to_path_buf();
    let mut result = FileVerification {
        path: path.clone(),
        expected_sha256: expected_sha256.to_ascii_lowercase(),
        actual_sha256: None,
        expected_bytes,
        actual_bytes: None,
        status: IntegrityStatus::Missing,
    };

    let metadata = match tokio::fs::metadata(&path).await {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => {
            result.status = IntegrityStatus::ReadError;
            return result;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return result,
        Err(_) => {
            result.status = IntegrityStatus::ReadError;
            return result;
        }
    };
    let size = metadata.len();
    result.actual_bytes = Some(size);

    match digest_file(&path).await {
        Ok(digest) => {
            result.actual_sha256 = Some(digest);
        }
        Err(_) => {
            result.status = IntegrityStatus::ReadError;
            return result;
        }
    }

    result.status = if size != expected_bytes {
        IntegrityStatus::SizeMismatch
    } else if result.actual_sha256.as_deref() != Some(&result.expected_sha256) {
        IntegrityStatus::HashMismatch
    } else {
        IntegrityStatus::Verified
    };
    result
}

pub async fn digest_file(path: impl AsRef<Path>) -> Result<String> {
    let mut file = tokio::fs::File::open(path.as_ref())
        .await
        .with_context(|| format!("cannot open {}", path.as_ref().display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub async fn verify_manifest(loaded: &LoadedManifest) -> Result<VerificationReport> {
    let artifact = verify_file(
        loaded.artifact_path(),
        &loaded.manifest.artifact.sha256,
        loaded.manifest.artifact.bytes,
    )
    .await;

    let backend = match &loaded.manifest.backend.sha256 {
        Some(digest) => {
            let path = loaded.backend_path();
            let bytes = tokio::fs::metadata(&path)
                .await
                .with_context(|| format!("cannot stat pinned backend {}", path.display()))?
                .len();
            Some(verify_file(path, digest, bytes).await)
        }
        None => None,
    };

    let backend_path = loaded.backend_path();
    let backend_directory = backend_path.parent().unwrap_or_else(|| Path::new("."));
    let mut backend_dependencies = Vec::new();
    for dependency in &loaded.manifest.backend.dependencies {
        let path = backend_directory.join(&dependency.path);
        let bytes = tokio::fs::metadata(&path)
            .await
            .with_context(|| format!("cannot stat backend dependency {}", path.display()))?
            .len();
        backend_dependencies.push(verify_file(path, &dependency.sha256, bytes).await);
    }
    let backend_dependencies_verified = backend_dependencies
        .iter()
        .all(|item| item.status == IntegrityStatus::Verified);

    let verified = artifact.status == IntegrityStatus::Verified
        && backend
            .as_ref()
            .map(|item| item.status == IntegrityStatus::Verified)
            .unwrap_or(true)
        && backend_dependencies_verified;

    Ok(VerificationReport {
        schema: "caleta.dev/verification/v1".into(),
        model: loaded.manifest.id.clone(),
        artifact,
        backend,
        backend_pinned: loaded.manifest.backend.sha256.is_some(),
        backend_dependencies,
        backend_dependencies_verified,
        verified,
        authenticity: "integrity_only; trust_requires_a_verified_manifest_source".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn verifies_content_and_size() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"caleta").unwrap();
        let expected = digest_file(file.path()).await.unwrap();
        let result = verify_file(file.path(), &expected, 6).await;
        assert_eq!(result.status, IntegrityStatus::Verified);
    }

    #[tokio::test]
    async fn detects_modified_content() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"caleta").unwrap();
        let result = verify_file(file.path(), &"0".repeat(64), 6).await;
        assert_eq!(result.status, IntegrityStatus::HashMismatch);
    }

    #[tokio::test]
    async fn reports_missing_file() {
        let result = verify_file("/definitely/not/here", &"0".repeat(64), 6).await;
        assert_eq!(result.status, IntegrityStatus::Missing);
    }
}
