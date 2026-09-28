#![cfg(unix)]
use mesh_llm_plugin::{LocalStream, PROTOCOL_VERSION, proto, read_envelope, write_envelope};
use std::{os::unix::fs::PermissionsExt, time::Duration};
use tokio::{net::UnixListener, process::Command};

#[tokio::test]
async fn host_disconnect_reaps_only_owned_backend() {
    check_cleanup(false).await;
}

#[tokio::test]
async fn host_kill_reaps_only_owned_backend() {
    check_cleanup(true).await;
}

async fn check_cleanup(hard_kill: bool) {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("ds4-server");
    std::fs::write(&executable, "#!/bin/sh\nexec sleep 60\n").unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
    let weights = directory.path().join("model.gguf");
    std::fs::write(&weights, b"fake").unwrap();
    // macOS Unix socket paths have a small bound; use /tmp rather than TMPDIR.
    let sockets = tempfile::tempdir_in("/tmp").unwrap();
    let socket = sockets.path().join("ipc");
    let listener = UnixListener::bind(&socket).unwrap();
    let mut unrelated = Command::new("sleep")
        .arg("60")
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_ds4"))
        .args(["serve", "--runtime"])
        .arg(directory.path())
        .arg("--weights")
        .arg(weights)
        .env("MESH_LLM_PLUGIN_ENDPOINT", &socket)
        .env("MESH_LLM_PLUGIN_TRANSPORT", "unix")
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let (stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut stream = LocalStream::Unix(stream);
    write_envelope(
        &mut stream,
        &proto::Envelope {
            protocol_version: PROTOCOL_VERSION,
            plugin_id: "ds4".into(),
            request_id: 1,
            payload: Some(proto::envelope::Payload::InitializeRequest(
                proto::InitializeRequest {
                    host_protocol_version: PROTOCOL_VERSION,
                    host_version: "test".into(),
                    host_info_json: "{}".into(),
                    mesh_visibility: proto::MeshVisibility::Private as i32,
                },
            )),
        },
    )
    .await
    .unwrap();
    let reply = tokio::time::timeout(Duration::from_secs(5), read_envelope(&mut stream))
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        reply.payload,
        Some(proto::envelope::Payload::InitializeResponse(_))
    ));
    tokio::time::sleep(Duration::from_millis(100)).await;
    let output = std::process::Command::new("pgrep")
        .args(["-P", &child.id().unwrap().to_string()])
        .output()
        .unwrap();
    let backend: u32 = String::from_utf8(output.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let output = std::process::Command::new("pgrep")
        .args(["-P", &backend.to_string()])
        .output()
        .unwrap();
    let server: u32 = String::from_utf8(output.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    if hard_kill {
        child.kill().await.unwrap();
    }
    drop(stream);
    tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .unwrap()
        .unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let alive = std::process::Command::new("kill")
        .args(["-0", &backend.to_string()])
        .output()
        .unwrap();
    assert!(!alive.status.success(), "owned backend survived disconnect");
    let alive = std::process::Command::new("kill")
        .args(["-0", &server.to_string()])
        .output()
        .unwrap();
    assert!(
        !alive.status.success(),
        "native backend survived plugin death"
    );
    assert!(unrelated.try_wait().unwrap().is_none());
    unrelated.kill().await.unwrap();
}

#[tokio::test]
async fn incompatible_host_never_starts_backend() {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("ds4-server");
    std::fs::write(&executable, "#!/bin/sh\ntouch started\nexec sleep 60\n").unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
    let weights = directory.path().join("model.gguf");
    std::fs::write(&weights, b"fake").unwrap();
    let sockets = tempfile::tempdir_in("/tmp").unwrap();
    let socket = sockets.path().join("ipc");
    let listener = UnixListener::bind(&socket).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_ds4"))
        .args(["serve", "--runtime"])
        .arg(directory.path())
        .arg("--weights")
        .arg(weights)
        .env("MESH_LLM_PLUGIN_ENDPOINT", &socket)
        .env("MESH_LLM_PLUGIN_TRANSPORT", "unix")
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let (stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut stream = LocalStream::Unix(stream);
    write_envelope(
        &mut stream,
        &proto::Envelope {
            protocol_version: PROTOCOL_VERSION,
            plugin_id: "ds4".into(),
            request_id: 1,
            payload: Some(proto::envelope::Payload::InitializeRequest(
                proto::InitializeRequest {
                    host_protocol_version: PROTOCOL_VERSION + 1,
                    host_version: "incompatible".into(),
                    host_info_json: "{}".into(),
                    mesh_visibility: proto::MeshVisibility::Private as i32,
                },
            )),
        },
    )
    .await
    .unwrap();
    let reply = tokio::time::timeout(Duration::from_secs(5), read_envelope(&mut stream))
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        reply.payload,
        Some(proto::envelope::Payload::ErrorResponse(_))
    ));
    drop(stream);
    tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .unwrap()
        .unwrap();
    assert!(
        !directory.path().join("started").exists(),
        "rejected host started a backend"
    );
}
