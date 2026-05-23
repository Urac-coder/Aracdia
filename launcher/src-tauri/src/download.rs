//! HTTP download + SHA-256 verification + zip extraction.
//!
//! Pure I/O helpers, no Tauri types so we can unit-test them in isolation.

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("checksum mismatch: expected {expected}, got {got}")]
    ChecksumMismatch { expected: String, got: String },
    #[error("server returned {status} for {url}")]
    BadStatus { status: u16, url: String },
    #[error("download cancelled")]
    Cancelled,
}

/// Reports progress while downloading. Implementors should be cheap (called
/// once per network chunk, ~64 KiB).
pub trait ProgressSink: Send + Sync {
    fn on_progress(&self, bytes_done: u64, bytes_total: Option<u64>);
}

impl<F> ProgressSink for F
where
    F: Fn(u64, Option<u64>) + Send + Sync,
{
    fn on_progress(&self, bytes_done: u64, bytes_total: Option<u64>) {
        self(bytes_done, bytes_total);
    }
}

/// Streams `url` to `dest`, reporting progress periodically. The destination's
/// parent directory is created on demand. Existing files at `dest` are
/// overwritten.
pub async fn download_to_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    progress: &dyn ProgressSink,
) -> Result<(), DownloadError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(DownloadError::BadStatus {
            status: response.status().as_u16(),
            url: url.to_owned(),
        });
    }

    let total = response.content_length();
    let mut stream = response.bytes_stream();

    let mut file = tokio::fs::File::create(dest).await?;
    let mut downloaded: u64 = 0;
    progress.on_progress(0, total);

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        progress.on_progress(downloaded, total);
    }

    file.flush().await?;
    file.sync_all().await?;
    Ok(())
}

/// Computes the hex-encoded SHA-256 of a file, in 64 KiB chunks.
pub fn sha256_file(path: &Path) -> Result<String, DownloadError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Verifies that the file at `path` hashes to `expected_hex` (case-insensitive).
pub fn verify_sha256(path: &Path, expected_hex: &str) -> Result<(), DownloadError> {
    let got = sha256_file(path)?;
    if !got.eq_ignore_ascii_case(expected_hex) {
        return Err(DownloadError::ChecksumMismatch {
            expected: expected_hex.to_owned(),
            got,
        });
    }
    Ok(())
}

/// Extracts a zip archive into `dest_dir`. The destination is wiped before
/// extraction so the install is always coherent.
///
/// Preserves Unix file permissions (so executables stay executable on macOS/Linux).
pub fn extract_zip(archive_path: &Path, dest_dir: &Path) -> Result<(), DownloadError> {
    if dest_dir.exists() {
        fs::remove_dir_all(dest_dir)?;
    }
    fs::create_dir_all(dest_dir)?;

    let bytes = fs::read(archive_path)?;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath: PathBuf = match file.enclosed_name() {
            Some(path) => dest_dir.join(path),
            // Skip entries with absolute or traversal-y paths (zip slip protection)
            None => continue,
        };

        if file.is_dir() {
            fs::create_dir_all(&outpath)?;
            continue;
        }

        if let Some(parent) = outpath.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut outfile = fs::File::create(&outpath)?;
        std::io::copy(&mut file, &mut outfile)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_tmp(name: &str, bytes: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("aracdia-test-{name}"));
        fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn sha256_known_value() {
        // SHA-256 of "abc" = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        let path = write_tmp("sha256-abc", b"abc");
        let got = sha256_file(&path).unwrap();
        assert_eq!(
            got,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        fs::remove_file(path).ok();
    }

    #[test]
    fn verify_sha256_detects_mismatch() {
        let path = write_tmp("sha256-mismatch", b"abc");
        let res = verify_sha256(&path, &"00".repeat(32));
        assert!(matches!(res, Err(DownloadError::ChecksumMismatch { .. })));
        fs::remove_file(path).ok();
    }

    #[test]
    fn verify_sha256_case_insensitive() {
        let path = write_tmp("sha256-case", b"abc");
        let upper = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD";
        verify_sha256(&path, upper).unwrap();
        fs::remove_file(path).ok();
    }
}
