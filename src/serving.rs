use anyhow::{Context, Result, bail, ensure};
use mesh_llm_plugin::{PluginMetadata, PluginRuntime, PluginStartupPolicy, plugin_server_info};
use std::{os::fd::AsFd, path::PathBuf, process::Stdio, time::Duration};
use tokio::process::{Child, Command};

/// `context == 0` means "use ds4-server's own default" (32768 at the pinned revision).
fn arguments(weights: &std::path::Path, context: u32, port: u16) -> Vec<String> {
    let mut args = vec!["-m".into(), weights.to_string_lossy().into_owned()];
    if context > 0 {
        args.extend(["--ctx".into(), context.to_string()]);
    }
    args.extend([
        "--host".into(),
        "127.0.0.1".into(),
        "--port".into(),
        port.to_string(),
    ]);
    args
}

/// What to serve: an explicit weight file, or a ds4 model name fetched by the
/// bundled upstream `download_model.sh` when missing.
pub enum Weights {
    Path(PathBuf),
    Model(String),
}

/// Default weight directory: outside the plugin install dir, so uninstall or
/// upgrade never deletes downloaded weights.
fn default_model_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(".mesh-llm/models/ds4"))
}

/// Runs upstream's downloader (resumes, verifies, and links `ds4flash.gguf` in
/// the runtime dir) and returns the resolved weight file. Cheap when present.
async fn fetch_model(
    runtime: &std::path::Path,
    model: &str,
    dir: &std::path::Path,
) -> Result<PathBuf> {
    let script = runtime.join("download_model.sh");
    ensure!(
        script.is_file(),
        "runtime is missing download_model.sh; reinstall the plugin"
    );
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    eprintln!(
        "ds4: ensuring model {model} in {} (downloads if missing; progress below)",
        dir.display()
    );
    let status = Command::new("/bin/sh")
        .arg(&script)
        .arg(model)
        .env("DS4_GGUF_DIR", dir)
        .current_dir(runtime)
        .stdin(Stdio::null())
        // Keep plugin stdout clean; everything goes to the Mesh terminal via stderr.
        .stdout(std::process::Stdio::from(
            std::io::stderr().as_fd().try_clone_to_owned()?,
        ))
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .status()
        .await
        .context("run bundled download_model.sh")?;
    ensure!(
        status.success(),
        "ds4: fetching {model} failed ({status}); fix the message above and restart Mesh (downloads resume)"
    );
    let link = runtime.join("ds4flash.gguf");
    let weights = link
        .canonicalize()
        .with_context(|| format!("{model} is not a servable main model (no ds4flash.gguf link)"))?;
    ensure!(
        weights.starts_with(dir.canonicalize()?),
        "{model} did not resolve into {}",
        dir.display()
    );
    Ok(weights)
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
            && body["data"]
                .as_array()
                .is_some_and(|items| !items.is_empty())
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    bail!("ds4-server did not list a model within the startup deadline; inspect server stderr")
}

fn plugin(name: &str, url: String) -> mesh_llm_plugin::SimplePlugin {
    mesh_llm_plugin::plugin! {
        metadata: PluginMetadata::new(name, env!("CARGO_PKG_VERSION"), plugin_server_info(
            name, env!("CARGO_PKG_VERSION"), "DwarfStar", "Managed local DwarfStar inference", None::<String>)),
        startup_policy: PluginStartupPolicy::Any,
        inference: [mesh_llm_plugin::inference::provider("ds4", url)],
    }
}

pub async fn run(
    runtime: PathBuf,
    weights: Weights,
    model_dir: Option<PathBuf>,
    context: u32,
    standalone: bool,
) -> Result<()> {
    ensure!(
        standalone || std::env::var_os("MESH_LLM_PLUGIN_ENDPOINT").is_some(),
        "serve must be launched by Mesh as a plugin; no server started"
    );
    let runtime = runtime
        .canonicalize()
        .context("runtime directory missing; provision pinned ds4 runtime first")?;
    let weights = match weights {
        Weights::Path(path) => {
            let path = path
                .canonicalize()
                .with_context(|| format!("weights not found: {}", path.display()))?;
            ensure!(path.is_file(), "weights must be a file");
            Weights::Path(path)
        }
        model => model,
    };
    let model_dir = match model_dir {
        Some(dir) => dir,
        None => default_model_dir()?,
    };
    let executable = runtime.join("ds4-server");
    ensure!(
        executable.is_file(),
        "runtime must contain ds4-server and its Metal assets"
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    let url = format!("http://127.0.0.1:{port}/v1");
    // Do not allocate a model process until a compatible host has initialized us.
    let initialized = std::sync::Arc::new(tokio::sync::Notify::new());
    let ready = initialized.clone();
    let executable = std::env::current_exe()?;
    let name = if executable.file_stem().and_then(|s| s.to_str()) == Some("ds4-plugin") {
        "ds4-plugin"
    } else {
        "ds4"
    };
    let plugin = plugin(name, url.clone())
        .on_initialize(|request, _| {
            Box::pin(async move {
                if request.host_protocol_version != mesh_llm_plugin::PROTOCOL_VERSION {
                    return Err(mesh_llm_plugin::PluginError::internal(format!(
                        "incompatible Mesh plugin protocol: host={}, plugin={}",
                        request.host_protocol_version,
                        mesh_llm_plugin::PROTOCOL_VERSION
                    )));
                }
                Ok(())
            })
        })
        .on_initialized(move |_| {
            ready.notify_one();
            Box::pin(async { Ok(()) })
        });
    let connection = PluginRuntime::run(plugin);
    tokio::pin!(connection);
    if !standalone {
        tokio::select! {
            result = &mut connection => return result,
            _ = initialized.notified() => {},
            _ = shutdown_signal() => return Ok(()),
            _ = tokio::time::sleep(Duration::from_secs(15)) => bail!("Mesh initialization timed out; no server started"),
        }
    }
    // Acquisition runs concurrently with the plugin connection, so Mesh health
    // checks keep answering Ok while a large download or verification runs; the
    // endpoint simply stays unready until the backend lists a model.
    let child_slot: std::sync::Arc<tokio::sync::Mutex<Option<Child>>> = Default::default();
    let serve = {
        let child_slot = child_slot.clone();
        let url = url.clone();
        async move {
            let weights = match weights {
                Weights::Path(path) => path,
                Weights::Model(model) => fetch_model(&runtime, &model, &model_dir).await?,
            };
            let child = Command::new(std::env::current_exe()?)
                .arg("watch-backend")
                .arg(&runtime)
                .arg(&weights)
                .arg(context.to_string())
                .arg(port.to_string())
                .arg(std::process::id().to_string())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .kill_on_drop(true)
                .spawn()
                .context("start owned ds4-server")?;
            let ctx = if context > 0 {
                context.to_string()
            } else {
                "server default".into()
            };
            eprintln!(
                "ds4: starting DwarfStar with {} at {url}; context={ctx}",
                weights.display()
            );
            let mut guard = child_slot.lock().await;
            let child = guard.insert(child);
            wait_ready(child, &url).await?;
            eprintln!("ds4: ready at {url}");
            let status = child.wait().await?;
            bail!("owned ds4-server exited: {status}")
        }
    };
    let result = if standalone {
        tokio::select! {
            result = serve => result,
            _ = shutdown_signal() => Ok(()),
        }
    } else {
        tokio::select! {
            result = &mut connection => result,
            result = serve => result,
            _ = shutdown_signal() => Ok(()),
        }
    };
    // Dropping `serve` cancelled any in-flight download (kill_on_drop; upstream
    // resumes the partial file next start). Only our own child is terminated.
    if let Some(mut child) = child_slot.lock().await.take()
        && child.try_wait()?.is_none()
    {
        if let Some(pid) = child.id() {
            // SAFETY: signaling only the supervisor child we own.
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }
        child.wait().await?;
    }
    result
}

/// A separate supervisor survives a host SIGKILL of the plugin and reaps its backend.
/// Parent identity is checked through the OS parent relationship, not PID-name matching.
pub async fn watch_backend(
    runtime: PathBuf,
    weights: PathBuf,
    context: u32,
    port: u16,
    parent: u32,
) -> Result<()> {
    #[cfg(unix)]
    let parent_alive = || {
        // SAFETY: getppid has no arguments or memory preconditions.
        unsafe { libc::getppid() as u32 == parent }
    };
    #[cfg(not(unix))]
    compile_error!("The trial backend watchdog currently requires Unix");
    ensure!(parent_alive(), "plugin already exited; no backend started");
    let mut child = Command::new(runtime.join("ds4-server"))
        .args(arguments(&weights, context, port))
        .current_dir(runtime)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()?;
    let result = tokio::select! {
        status = child.wait() => { bail!("ds4-server exited: {}", status?) },
        _ = async {
            while parent_alive() {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        } => Ok(()),
        _ = shutdown_signal() => Ok(()),
    };
    if child.try_wait()?.is_none() {
        child.kill().await?;
    }
    result
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
        // No --ctx at all by default: ds4-server picks its own (32768).
        let args = arguments(std::path::Path::new("/m.gguf"), 0, 1);
        assert!(!args.iter().any(|a| a == "--ctx"));
    }

    #[test]
    fn manifest_uses_existing_managed_provider_contract() {
        let p = plugin("ds4-plugin", "http://127.0.0.1:12345/v1".into());
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
