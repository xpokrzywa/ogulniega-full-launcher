//! Pobieranie plików: jeden klient HTTP na proces, streaming, resume, hash na bieżąco.
//!
//! Ciało odpowiedzi nigdy nie ląduje w RAM w całości — idzie chunkami do
//! `BufWriter` i równolegle do `Sha512`. Semafor ogranicza liczbę równoległych
//! transferów, żeby pula połączeń nie rozlała się przy wielu plikach naraz.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use sha2::{Digest, Sha512};
use tokio::io::{AsyncSeekExt, AsyncWriteExt, BufWriter};
use tokio::sync::Semaphore;

use crate::error::AppError;

const BUF_SIZE: usize = 64 * 1024;
const MAX_ATTEMPTS: u32 = 3;
const BACKOFF_BASE: Duration = Duration::from_millis(500);

pub struct Downloader {
    client: reqwest::Client,
    sem: Arc<Semaphore>,
}

/// Wynik pobrania: rozmiar i faktyczny SHA-512 pliku na dysku.
pub struct DownloadOutcome {
    pub size: u64,
    pub sha512: String,
}

impl Downloader {
    pub fn new() -> Result<Self, AppError> {
        let client = reqwest::Client::builder()
            .user_agent("ogulniega-full-launcher/1.0")
            .https_only(true)
            .timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(8)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;
        Ok(Self {
            client,
            sem: Arc::new(Semaphore::new(16)),
        })
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Pobiera `url` do `dest`.
    ///
    /// Gdy plik już istnieje i `expected_sha512` się zgadza, nie rusza sieci.
    /// Zapis idzie do `<dest>.part` i dopiero po weryfikacji jest atomowo
    /// przemianowywany — przerwany transfer nie zostawia niepełnego pliku
    /// pod docelową nazwą.
    pub async fn download_to(
        &self,
        url: &str,
        dest: &Path,
        expected_sha512: Option<&str>,
    ) -> Result<DownloadOutcome, AppError> {
        if let Some(parent) = dest.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        if dest.exists() {
            if let Some(expected) = expected_sha512 {
                let got = sha512_file(dest).await?;
                if hash_eq(&got, expected) {
                    let len = tokio::fs::metadata(dest).await?.len();
                    return Ok(DownloadOutcome { size: len, sha512: got });
                }
            }
        }

        let part = part_path(dest);
        let mut last_err = AppError::Http("brak próby pobrania".to_string());

        for attempt in 0..MAX_ATTEMPTS {
            if attempt > 0 {
                tokio::time::sleep(BACKOFF_BASE * 2u32.pow(attempt - 1)).await;
            }
            match self.try_once(url, dest, &part, expected_sha512).await {
                Ok(len) => return Ok(len),
                Err(err) if is_retryable(&err) && attempt + 1 < MAX_ATTEMPTS => {
                    last_err = err;
                }
                Err(err) => return Err(err),
            }
        }
        Err(last_err)
    }

    async fn try_once(
        &self,
        url: &str,
        dest: &Path,
        part: &Path,
        expected_sha512: Option<&str>,
    ) -> Result<DownloadOutcome, AppError> {
        let _permit = self
            .sem
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| AppError::Io(e.to_string()))?;

        let existing = match tokio::fs::metadata(part).await {
            Ok(meta) if meta.len() > 0 => meta.len(),
            _ => 0,
        };

        let mut request = self.client.get(url);
        if existing > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={existing}-"));
        }

        let response = request.send().await?;
        let status = response.status();

        if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(AppError::Http(format!("HTTP {status} dla {url}")));
        }

        let resume = status == reqwest::StatusCode::PARTIAL_CONTENT && existing > 0;
        if !resume && existing > 0 {
            // 200 zamiast 206: serwer zignorował Range, prefiks jest bezużyteczny.
            tokio::fs::remove_file(part).await.ok();
        }

        let prefix = if resume { existing } else { 0 };
        let mut hasher = Sha512::new();
        if prefix > 0 {
            hash_prefix(part, prefix, &mut hasher).await?;
        }

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(prefix == 0)
            .open(part)
            .await?;
        if prefix > 0 {
            // OpenOptions nie ustawia pozycji zapisu — bez seek dopisalibyśmy od zera.
            file.set_len(prefix).await?;
            file.seek(std::io::SeekFrom::Start(prefix)).await?;
        }
        let mut writer = BufWriter::with_capacity(BUF_SIZE, file);

        let mut stream = response.bytes_stream();
        let mut written = prefix;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| AppError::Http(e.to_string()))?;
            hasher.update(&chunk);
            writer.write_all(&chunk).await?;
            written += chunk.len() as u64;
        }
        writer.flush().await?;
        writer.get_ref().sync_all().await?;
        drop(writer);

        if let Some(expected) = expected_sha512 {
            let got = hex::encode(hasher.finalize());
            if !hash_eq(&got, expected) {
                tokio::fs::remove_file(part).await.ok();
                return Err(AppError::HashMismatch {
                    expected: expected.to_string(),
                    got,
                    path: dest.display().to_string(),
                });
            }
            tokio::fs::rename(part, dest).await?;
            return Ok(DownloadOutcome { size: written, sha512: got });
        }

        let got = hex::encode(hasher.finalize());
        tokio::fs::rename(part, dest).await?;
        Ok(DownloadOutcome {
            size: written,
            sha512: got,
        })
    }
}

fn part_path(dest: &Path) -> PathBuf {
    let mut name = dest
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".part");
    dest.with_file_name(name)
}

fn hash_eq(got: &str, expected: &str) -> bool {
    got.eq_ignore_ascii_case(expected.trim())
}

fn is_retryable(err: &AppError) -> bool {
    match err {
        AppError::Io(_) => true,
        AppError::Http(msg) => {
            let lower = msg.to_ascii_lowercase();
            lower.contains("timed out")
                || lower.contains("timeout")
                || lower.contains("connection")
                || lower.contains("http 5")
        }
        _ => false,
    }
}

/// Hash istniejącego prefiksu `.part` przed dopisaniem reszty.
async fn hash_prefix(path: &Path, len: u64, hasher: &mut Sha512) -> Result<(), AppError> {
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path).await?;
    let mut left = len;
    let mut buf = vec![0u8; BUF_SIZE];
    while left > 0 {
        let want = std::cmp::min(left, buf.len() as u64) as usize;
        let read = file.read(&mut buf[..want]).await?;
        if read == 0 {
            return Err(AppError::Io(format!(
                "plik krótszy niż {len} B: {}",
                path.display()
            )));
        }
        hasher.update(&buf[..read]);
        left -= read as u64;
    }
    Ok(())
}

async fn sha512_file(path: &Path) -> Result<String, AppError> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || sha512_file_blocking(&path))
        .await
        .map_err(|e| AppError::Io(e.to_string()))?
}

/// SHA-512 pliku, bufor 64 KiB. Wołać z `spawn_blocking` — nie blokuje runtime.
pub fn sha512_file_blocking(path: &Path) -> Result<String, AppError> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha512::new();
    let mut buf = [0u8; BUF_SIZE];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha512_pliku_tymczasowego() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("probe.bin");
        std::fs::write(&path, b"ogulniega").unwrap();
        let got = sha512_file_blocking(&path).unwrap();
        let mut hasher = Sha512::new();
        hasher.update(b"ogulniega");
        assert_eq!(got, hex::encode(hasher.finalize()));
    }

    #[test]
    fn hash_ignoruje_wielkosc_liter() {
        assert!(hash_eq("AbC", "abc"));
        assert!(!hash_eq("abc", "abd"));
    }
}