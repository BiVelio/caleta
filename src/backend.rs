use crate::manifest::LoadedManifest;
use anyhow::{bail, Context, Result};
use rand::{rngs::OsRng, RngCore};
use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};

#[derive(Debug, Clone)]
pub struct BackendConfig {
    pub startup_timeout: Duration,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            startup_timeout: Duration::from_secs(120),
        }
    }
}

pub struct Backend {
    child: Child,
    pub base_url: String,
    pub api_key: String,
    pub executable: PathBuf,
    pub started_in: Duration,
    pub alive: Arc<AtomicBool>,
}

impl Backend {
    pub async fn spawn(loaded: &LoadedManifest, config: BackendConfig) -> Result<Self> {
        let started = Instant::now();
        let executable = loaded.backend_path();
        let port = reserve_loopback_port()?;
        let mut secret = [0_u8; 32];
        OsRng.fill_bytes(&mut secret);
        let api_key = hex::encode(secret);
        let runtime = &loaded.manifest.runtime;

        let mut command = Command::new(&executable);
        // Environment-based llama.cpp options could otherwise re-enable tools,
        // add accepted API keys, or override the boundary declared by Caleta.
        for (key, _) in std::env::vars_os() {
            let name = key.to_string_lossy();
            if name.starts_with("LLAMA_ARG_") || name == "LLAMA_API_KEY" {
                command.env_remove(key);
            }
        }
        command
            .arg("--model")
            .arg(loaded.artifact_path())
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .arg("--alias")
            .arg(&loaded.manifest.id)
            .arg("--offline")
            .arg("--no-webui")
            .arg("--ctx-size")
            .arg(runtime.context_size.to_string())
            .arg("--n-gpu-layers")
            .arg(runtime.gpu_layers.to_string())
            // Keep credentials out of the command line and process listings.
            .env("LLAMA_API_KEY", &api_key)
            .env("LLAMA_ARG_LOG_VERBOSITY", "1")
            .args(&loaded.manifest.backend.args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        if let Some(threads) = runtime.threads {
            command.arg("--threads").arg(threads.to_string());
        }

        let mut child = command.spawn().with_context(|| {
            format!(
                "cannot start backend {}; install llama-server or set backend.executable",
                executable.display()
            )
        })?;
        let base_url = format!("http://127.0.0.1:{port}");
        let client = reqwest::Client::builder().no_proxy().build()?;
        let deadline = Instant::now() + config.startup_timeout;

        loop {
            if let Some(status) = child.try_wait()? {
                bail!("backend exited before becoming ready: {status}");
            }
            let ready = client
                .get(format!("{base_url}/health"))
                .bearer_auth(&api_key)
                .timeout(Duration::from_secs(2))
                .send()
                .await;
            if matches!(ready, Ok(response) if response.status().is_success()) {
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill().await;
                bail!(
                    "backend did not become ready within {} seconds",
                    config.startup_timeout.as_secs()
                );
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
        }

        Ok(Self {
            child,
            base_url,
            api_key,
            executable,
            started_in: started.elapsed(),
            alive: Arc::new(AtomicBool::new(true)),
        })
    }

    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    pub async fn wait(&mut self) -> Result<ExitStatus> {
        let status = self.child.wait().await?;
        self.alive.store(false, Ordering::SeqCst);
        Ok(status)
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        if self.child.try_wait()?.is_none() {
            self.child.kill().await?;
        }
        self.child.wait().await?;
        self.alive.store(false, Ordering::SeqCst);
        Ok(())
    }

    pub async fn version(executable: &PathBuf) -> Option<String> {
        let output = tokio::time::timeout(
            Duration::from_secs(10),
            Command::new(executable).arg("--version").output(),
        )
        .await
        .ok()?
        .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = if output.stdout.is_empty() {
            output.stderr
        } else {
            output.stdout
        };
        Some(String::from_utf8_lossy(&text).trim().to_string())
    }
}

fn reserve_loopback_port() -> Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}
