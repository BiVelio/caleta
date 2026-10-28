use crate::backend::Backend;
use crate::manifest::LoadedManifest;
use crate::verify::verify_manifest;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub healthy: bool,
    pub caleta_version: String,
    pub os: String,
    pub arch: String,
    pub checks: Vec<DoctorCheck>,
}

#[derive(Debug, Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

pub async fn doctor(manifest: Option<PathBuf>, backend: Option<PathBuf>) -> DoctorReport {
    let mut checks = Vec::new();
    let loaded = manifest.and_then(|path| match LoadedManifest::load(path) {
        Ok(loaded) => {
            checks.push(DoctorCheck {
                name: "manifest".into(),
                passed: true,
                detail: format!("valid manifest for {}", loaded.manifest.id),
            });
            Some(loaded)
        }
        Err(error) => {
            checks.push(DoctorCheck {
                name: "manifest".into(),
                passed: false,
                detail: format!("{error:#}"),
            });
            None
        }
    });

    if let Some(loaded) = &loaded {
        match verify_manifest(loaded).await {
            Ok(report) if report.verified => checks.push(DoctorCheck {
                name: "artifact_integrity".into(),
                passed: true,
                detail: "model artifact matches its declared size and SHA-256".into(),
            }),
            Ok(report) => checks.push(DoctorCheck {
                name: "artifact_integrity".into(),
                passed: false,
                detail: format!("artifact status: {:?}", report.artifact.status),
            }),
            Err(error) => checks.push(DoctorCheck {
                name: "artifact_integrity".into(),
                passed: false,
                detail: format!("{error:#}"),
            }),
        }
    }

    let executable = backend
        .or_else(|| loaded.as_ref().map(LoadedManifest::backend_path))
        .unwrap_or_else(|| "llama-server".into());
    match Backend::version(&executable).await {
        Some(version) => checks.push(DoctorCheck {
            name: "backend".into(),
            passed: true,
            detail: format!("{}: {version}", executable.display()),
        }),
        None => checks.push(DoctorCheck {
            name: "backend".into(),
            passed: false,
            detail: format!(
                "cannot execute {} --version; install llama.cpp or configure backend.executable",
                executable.display()
            ),
        }),
    }

    let healthy = checks.iter().all(|check| check.passed);
    DoctorReport {
        healthy,
        caleta_version: crate::VERSION.into(),
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        checks,
    }
}
