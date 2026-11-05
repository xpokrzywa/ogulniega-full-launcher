use std::io::Read;
use std::time::Duration;

use serde_json::json;
use sha2::{Digest, Sha512};

pub const MAX_BYTES: u64 = 8 * 1024 * 1024;

const ALLOWED_HOSTS: [&str; 4] = [
    "https://cdn.jsdelivr.net/",
    "https://api.modrinth.com/",
    "https://ogulniega.com/",
    "https://github.com/",
];

const MIRROR: &str = "https://cdn.jsdelivr.net/gh/xMelonekMaX/ogulniega-distribution@main/";

pub fn check_url(url: &str) -> Result<(), String> {
    if ALLOWED_HOSTS.iter().any(|prefix| url.starts_with(prefix)) {
        Ok(())
    } else {
        Err(format!("adres niedozwolony: {url}"))
    }
}

pub fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(60))
        .user_agent("ogulniega-full-launcher/1.0 (+https://github.com/xpokrzywa/ogulniega-mobile)")
        .build()
}

/// Pobiera tekst (JSON) i zwraca go do front-endu.
#[tauri::command]
pub fn fetch_json(url: String) -> Result<String, String> {
    check_url(&url)?;
    let response = agent().get(&url).call().map_err(|e| e.to_string())?;
    let mut body = String::new();
    response
        .into_reader()
        .take(MAX_BYTES)
        .read_to_string(&mut body)
        .map_err(|e| e.to_string())?;
    Ok(body)
}

/// Lista wersji z `launcher.json` (backend pełnego launchera).
#[tauri::command]
pub fn list_versions() -> Result<String, String> {
    let body = fetch_json(format!("{MIRROR}launcher.json"))?;
    let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let versions: Vec<serde_json::Value> = parsed
        .get("versions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    serde_json::to_string(&json!({ "selected_version": parsed.get("selected_version"), "versions": versions }))
        .map_err(|e| e.to_string())
}

/// Pobiera plik (klient JVM / mod jar) na dysk z ewentualną progresją? Prosta wersja:
/// zapis do docelowej ścieżki, zwraca sha512 + rozmiar.
#[tauri::command]
pub fn download_file(url: String, dest_path: String) -> Result<serde_json::Value, String> {
    check_url(&url)?;
    let response = agent().get(&url).call().map_err(|e| e.to_string())?;
    let mut reader = response.into_reader().take(512 * 1024 * 1024);
    let mut hasher = Sha512::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut file = std::fs::File::create(&dest_path).map_err(|e| e.to_string())?;
    let mut total = 0u64;
    loop {
        let read = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if read == 0 { break; }
        hasher.update(&buffer[..read]);
        total += read as u64;
        std::io::Write::write_all(&mut file, &buffer[..read]).map_err(|e| e.to_string())?;
    }
    let sha = hex::encode(hasher.finalize());
    Ok(json!({ "sha512": sha, "size": total, "path": dest_path }))
}

/// Liczy SHA-512 z pliku pod adresem URL, bez zapisu na dysk.
#[tauri::command]
pub fn sha512_of(url: String) -> Result<String, String> {
    check_url(&url)?;
    let response = agent().get(&url).call().map_err(|e| e.to_string())?;
    let mut reader = response.into_reader().take(256 * 1024 * 1024);
    let mut hasher = Sha512::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if read == 0 { break; }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Informacje o środowisku — pokazywane w zakładce „Start".
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
        ] {
            assert!(check_url(url).is_ok(), "{url} powinno przechodzić");
        }
    }

    #[test]
    fn nieznane_hosty_odrzuca() {
        for url in ["https://evil.example.com/x", "http://cdn.jsdelivr.net/x", "file:///etc/passwd"] {
            assert!(check_url(url).is_err(), "{url} powinno być odrzucone");
        }
    }
}

