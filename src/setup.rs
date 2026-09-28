use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};

pub const BYTES: u64 = 86_720_111_488;
const FILE: &str = "DeepSeek-V4-Flash-IQ2XXS-w2Q2K-AProjQ8-SExpQ8-OutQ8-chat-v2-imatrix-0731.gguf";
const SHA: &str = "ca22ae2f838e14077c22bc1c1417b71b45b5e5a3687bd96c2ac6e17fdb6261c0";
const REVISION: &str = "f71f23d552d664e523b422157b2befbf74040380";

fn verify(path: &Path, bytes: u64, expected: &str) -> Result<()> {
    let mut file = File::open(path)?;
    ensure!(
        file.metadata()?.len() == bytes,
        "wrong file size; expected {bytes} bytes"
    );
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    ensure!(
        format!("{:x}", hash.finalize()) == expected,
        "SHA-256 mismatch; file retained for inspection, not activated"
    );
    Ok(())
}

pub async fn download(directory: &Path, approved: bool) -> Result<()> {
    ensure!(
        approved,
        "Download requires --accept-download after reviewing ds4 catalog (81 GiB and model licence)"
    );
    std::fs::create_dir_all(directory)?;
    let directory = directory.canonicalize()?;
    let destination = directory.join(FILE);
    let lock = directory.join(".ds4-download.lock");
    let guard = File::options().write(true).create_new(true).open(&lock)
        .context("download lock exists or directory is unwritable; never remove a lock while another download runs")?;
    let result = download_locked(&destination).await;
    drop(guard);
    std::fs::remove_file(lock)?;
    result
}

async fn download_locked(destination: &Path) -> Result<()> {
    if destination.exists() {
        return verify(destination, BYTES, SHA)
            .context("existing weights failed verification; not overwriting");
    }
    let partial = destination.with_extension("gguf.partial");
    let url = format!("https://huggingface.co/antirez/deepseek-v4-gguf/resolve/{REVISION}/{FILE}");
    eprintln!(
        "Downloading {BYTES} bytes to {}; cancel retains partial file. Requires curl on PATH.",
        partial.display()
    );
    let status = tokio::process::Command::new("curl")
        .args([
            "--fail",
            "--location",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--continue-at",
            "-",
            "--output",
        ])
        .arg(&partial)
        .arg(url)
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .status()
        .await
        .context("launch curl for resumable weight download")?;
    ensure!(
        status.success(),
        "download failed; rerun to resume retained partial file"
    );
    verify(&partial, BYTES, SHA)?;
    std::fs::rename(partial, destination)?;
    eprintln!(
        "Verified weights: {}. Nothing has been started.",
        destination.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_rejects_size_and_hash_mismatch() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("weights");
        std::fs::write(&p, b"abc").unwrap();
        let hash = format!("{:x}", Sha256::digest(b"abc"));
        assert!(verify(&p, 3, &hash).is_ok());
        assert!(verify(&p, 4, &hash).is_err());
        assert!(verify(&p, 3, SHA).is_err());
    }

    #[tokio::test]
    async fn approval_required_before_creating_directory() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("absent");
        assert!(download(&p, false).await.is_err());
        assert!(!p.exists());
    }
}
