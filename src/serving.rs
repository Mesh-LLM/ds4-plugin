use anyhow::{Context, Result, bail, ensure};
use mesh_llm_plugin::{PluginMetadata, PluginRuntime, PluginStartupPolicy, plugin_server_info};
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::process::{Child, Command};

fn arguments(weights: &std::path::Path, context: u32, port: u16) -> Vec<String> {
    vec![
        "-m".into(),
        weights.to_string_lossy().into_owned(),
        "--ctx".into(),
        context.to_string(),
        "--host".into(),
        "127.0.0.1".into(),
        "--port".into(),
        port.to_string(),
    ]
}

async fn wait_ready(child: &mut Child, url: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;
    for _ in 0..300 {
        if let Some(status) = child.try_wait()? {
            bail!("ds4-server exited during startup: {status}");
        }
        if let Ok(response) = client.get(format!("{url}/models")).send().await
            && response.status().is_success()
            && let Ok(body) = response.json::<serde_json::Value>().await
            && body["data"].as_array().is_some_and(|items| {
                items
                    .iter()
                    .any(|m| m["id"].as_str() == Some("deepseek-v4-flash"))
            })
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    bail!(
        "ds4-server did not expose DeepSeek V4 Flash within startup deadline; inspect server stderr"
    )
}

fn plugin(url: String) -> mesh_llm_plugin::SimplePlugin {
    mesh_llm_plugin::plugin! {
        metadata: PluginMetadata::new("ds4", env!("CARGO_PKG_VERSION"), plugin_server_info(
            "ds4", env!("CARGO_PKG_VERSION"), "DwarfStar", "Managed local DwarfStar inference", None::<String>)),
        startup_policy: PluginStartupPolicy::Any,
        inference: [mesh_llm_plugin::inference::provider("ds4", url)],
    }
}

pub async fn run(runtime: PathBuf, weights: PathBuf, context: u32, standalone: bool) -> Result<()> {
    ensure!(
        standalone || std::env::var_os("MESH_LLM_PLUGIN_ENDPOINT").is_some(),
        "serve must be launched by Mesh as a plugin; no server started"
    );
    let runtime = runtime
        .canonicalize()
        .context("runtime directory missing; provision pinned ds4 runtime first")?;
    let weights = weights
        .canonicalize()
        .context("weights missing; use explicit download or supply existing weights")?;
    ensure!(weights.is_file(), "weights must be a file");
    let executable = runtime.join("ds4-server");
    ensure!(
        executable.is_file(),
        "runtime must contain ds4-server and its Metal assets"
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    let url = format!("http://127.0.0.1:{port}/v1");
    let mut child = Command::new(executable)
        .args(arguments(&weights, context, port))
        .current_dir(runtime)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .context("start owned ds4-server")?;
    eprintln!("Starting DwarfStar at {url}; one session, context={context}");
    let result = if standalone {
        tokio::select! {
            result = async {
                wait_ready(&mut child, &url).await?;
                eprintln!("Ready: {url} (upstream Flash/PRO IDs are aliases, not two loaded models)");
                bail!("ds4-server exited: {}", child.wait().await?)
            } => result,
            _ = shutdown_signal() => Ok(()),
        }
    } else {
        supervise(&mut child, url).await
    };
    // Only the Child handle we created is ever terminated. No process-name matching.
    if child.try_wait()?.is_none() {
        child.kill().await?;
    }
    result
}

async fn supervise(child: &mut Child, url: String) -> Result<()> {
    // Connect promptly: model loading must not consume the host's IPC handshake deadline.
    // Host endpoint probing keeps an unready backend out of routing.
    tokio::select! {
        result = PluginRuntime::run(plugin(url.clone())) => result,
        result = async {
            wait_ready(child, &url).await?;
            bail!("owned ds4-server exited: {}", child.wait().await?)
        } => result,
        _ = shutdown_signal() => Ok(()),
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        if let Ok(mut terminate) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            tokio::select! {
                _ = terminate.recv() => {},
                _ = tokio::signal::ctrl_c() => {},
            }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use mesh_llm_plugin::Plugin;

    #[test]
    fn owned_command_is_loopback_single_session_without_download() {
        let args = arguments(std::path::Path::new("/a b/model.gguf"), 4096, 12345);
        assert_eq!(
            args,
            [
                "-m",
                "/a b/model.gguf",
                "--ctx",
                "4096",
                "--host",
                "127.0.0.1",
                "--port",
                "12345"
            ]
        );
    }

    #[test]
    fn manifest_uses_existing_managed_provider_contract() {
        let p = plugin("http://127.0.0.1:12345/v1".into());
        let manifest = p.manifest().unwrap();
        assert_eq!(manifest.endpoints.len(), 1);
        let endpoint = &manifest.endpoints[0];
        assert!(endpoint.managed_by_plugin);
        assert_eq!(endpoint.protocol.as_deref(), Some("openai_compatible"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn exited_child_fails_readiness_without_starting_model() {
        let mut child = Command::new("/usr/bin/false").spawn().unwrap();
        child.wait().await.unwrap();
        assert!(
            wait_ready(&mut child, "http://127.0.0.1:1/v1")
                .await
                .is_err()
        );
    }
}
