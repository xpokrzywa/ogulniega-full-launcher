//! Komendy Tauri wołane z `www/app.js`.
//!
//! Nazwy i kształt wyników są kontraktem z front-endem: `fetch_json`,
//! `list_versions`, `download_file`, `sha512_of`, `backend_info`.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use serde_json::json;
use tauri::State;

use crate::download::{sha512_file_blocking, Downloader};
use crate::error::AppError;

/// Limit odpowiedzi JSON — manifesty są małe, chroni przed zalaniem pamięci.
const MAX_JSON_BYTES: usize = 8 * 1024 * 1024;

const ALLOWED_HOSTS: &[&str] = &[
    "cdn.jsdelivr.net",
    "ogulniega.com",
    "launchermeta.mojang.com",
    "launcher.mojang.com",
    "libraries.minecraft.net",
    "resources.download.minecraft.net",
    "piston-data.mojang.com",
    "maven.fabricmc.net",
    "meta.fabricmc.net",
    "cdn.modrinth.com",
    "api.modrinth.com",
    "api.curseforge.com",
    "edge.forgecdn.net",
    "github.com",
    "objects.githubusercontent.com",
    "raw.githubusercontent.com",
];

pub const MIRROR: &str = "https://cdn.jsdelivr.net/gh/xMelonekMaX/ogulniega-distribution@main/";

pub struct AppState {
    pub downloader: Arc<Downloader>,
    /// Katalog danych aplikacji — jedyne miejsce, do którego wolno pisać.
    pub data_dir: PathBuf,
}

/// Allowlista hostów. Zwraca sparsowany URL, żeby wołający nie parsował drugi raz.
pub fn check_url(url: &str) -> Result<String, AppError> {
    let parsed = reqwest::Url::parse(url).map_err(|e| AppError::BadUrl(e.to_string()))?;
    if parsed.scheme() != "https" {
        return Err(AppError::BadUrl(format!("wymagany https: {url}")));
    }
    let host = parsed.host_str().ok_or_else(|| AppError::BadUrl(url.to_string()))?;
    if ALLOWED_HOSTS.iter().any(|allowed| host.eq_ignore_ascii_case(allowed)) {
        Ok(parsed.to_string())
    } else {
        Err(AppError::BadHost(host.to_string()))
    }
}

/// Mapuje ścieżkę z UI na dysk w bazie katalogu danych aplikacji.
///
/// Względne dopisywane do `base`; absolutne dozwolone wyłącznie wewnątrz `base`.
/// `..` i `file://` odrzucone w obu przypadkach (klasa buga z Prism 11.1.1).
pub fn check_dest_path(base: &Path, dest: &str) -> Result<PathBuf, AppError> {
    let trimmed = dest.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadPath("pusta ścieżka".to_string()));
    }
    if trimmed.to_ascii_lowercase().starts_with("file:") {
        return Err(AppError::BadPath(format!("file:// niedozwolone: {dest}")));
    }
    let path = Path::new(trimmed);
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(AppError::BadPath(format!("`..` niedozwolone: {dest}")));
    }
    let joined = if path.is_absolute() {
        if !path.starts_with(base) {
            return Err(AppError::BadPath(format!(
                "ścieżka poza katalogiem danych: {dest}"
            )));
        }
        path.to_path_buf()
    } else {
        base.join(path)
    };
    if joined.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(AppError::BadPath(format!("`..` niedozwolone: {dest}")));
    }
    Ok(joined)
}

/// GET JSON z dozwolonego hosta. Ciało obcięte do `MAX_JSON_BYTES`.
#[tauri::command]
pub async fn fetch_json(
    state: State<'_, AppState>,
    url: String,
) -> Result<serde_json::Value, AppError> {
    let url = check_url(&url)?;
    let response = state.downloader.client().get(&url).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::Http(format!("HTTP {status} dla {url}")));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if bytes.len() > MAX_JSON_BYTES {
        return Err(AppError::Http(format!(
            "odpowiedź większa niż {MAX_JSON_BYTES} B"
        )));
    }
    Ok(serde_json::from_slice(&bytes)?)
}

/// Lista wersji z `launcher.json` na mirrorze dystrybucji.
#[tauri::command]
pub async fn list_versions(state: State<'_, AppState>) -> Result<serde_json::Value, AppError> {
    let parsed = fetch_json(state, format!("{MIRROR}launcher.json")).await?;
    let versions = parsed
        .get("versions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(json!({
        "selected_version": parsed.get("selected_version"),
        "versions": versions,
    }))
}

/// Pobiera plik na dysk. `expected_sha512` pomija sieć, gdy plik już się zgadza,
/// i odrzuca pobranie przy niezgodnym hashu.
#[tauri::command]
pub async fn download_file(
    state: State<'_, AppState>,
    url: String,
    dest_path: String,
    expected_sha512: Option<String>,
) -> Result<serde_json::Value, AppError> {
    let url = check_url(&url)?;
    let dest = check_dest_path(&state.data_dir, &dest_path)?;
    let expected = expected_sha512.as_deref().filter(|s| !s.is_empty());
    let out = state.downloader.download_to(&url, &dest, expected).await?;
    Ok(json!({
        "size": out.size,
        "path": dest.display().to_string(),
        "sha512": out.sha512,
    }))
}

/// SHA-512 pliku na dysku (nie URL — liczenie idzie przez `spawn_blocking`).
#[tauri::command]
pub async fn sha512_of(state: State<'_, AppState>, path: String) -> Result<String, AppError> {
    let path = check_dest_path(&state.data_dir, &path)?;
    tokio::task::spawn_blocking(move || sha512_file_blocking(&path))
        .await
        .map_err(|e| AppError::Io(e.to_string()))?
}

/// Informacje o środowisku — zakładka „Start".
#[tauri::command]
pub fn backend_info() -> serde_json::Value {
    json!({
        "backend": "rust",
        "appVersion": env!("CARGO_PKG_VERSION"),
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dozwolone_hosty_przyjmuja() {
        for url in [
            "https://cdn.jsdelivr.net/gh/xMelonekMaX/ogulniega-distribution@main/launcher.json",
            "https://api.modrinth.com/v2/search?query=sodium",
            "https://ogulniega.com/files/launcher.json",
            "https://github.com/xMelonekMaX/ogulniega-distribution/releases/download/x.jar",
            "https://cdn.modrinth.com/data/AANobbMI/versions/abc/sodium.jar",
            "https://meta.fabricmc.net/v2/versions/loader",
            "https://piston-data.mojang.com/v1/objects/ab/cd",
            "https://libraries.minecraft.net/com/mojang/patchy/1.3.9/patchy-1.3.9.jar",
            "https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.16.9/fabric-loader-0.16.9.jar",
            "https://edge.forgecdn.net/files/123/456/mod.jar",
            "https://raw.githubusercontent.com/x/y/main/README.md",
        ] {
            assert!(check_url(url).is_ok(), "{url} powinno przechodzić");
        }
    }

    #[test]
    fn nieznane_hosty_i_schematy_odrzuca() {
        for url in [
            "https://evil.example.com/x",
            "http://cdn.jsdelivr.net/x",
            "file:///etc/passwd",
            "ftp://cdn.jsdelivr.net/x",
            "https://cdn.jsdelivr.net.evil.com/x",
        ] {
            assert!(check_url(url).is_err(), "{url} powinno być odrzucone");
        }
    }

    #[test]
    fn sciezka_odrzuca_parent_i_absolutna_poza_baza() {
        let base = Path::new("/data/app");
        assert!(check_dest_path(base, "../etc/passwd").is_err());
        assert!(check_dest_path(base, "mods/../../secret").is_err());
        assert!(check_dest_path(base, "/etc/passwd").is_err());
        assert!(check_dest_path(base, "/data/app/../etc/passwd").is_err());
        assert!(check_dest_path(base, "file:///tmp/x").is_err());
        assert!(check_dest_path(base, "").is_err());
    }

    #[test]
    fn sciezka_wzgledna_trafia_do_bazy() {
        let base = Path::new("/data/app");
        assert_eq!(
            check_dest_path(base, "mods/sodium.jar").unwrap(),
            base.join("mods/sodium.jar")
        );
        // absolutna wewnątrz bazy jest dozwolona
        assert_eq!(
            check_dest_path(base, "/data/app/x.jar").unwrap(),
            base.join("x.jar")
        );
    }
}