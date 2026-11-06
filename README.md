# Ogulniega Full Launcher

Nieoficjalny launcher Minecraft (Java / Fabric) w Rust i Tauri 2: lista wersji z lustra Ogulniegi, przycisk Graj, pobieranie klienta JVM i modów z weryfikacją SHA-512.

[![Rust / Tauri](https://github.com/xpokrzywa/ogulniega-full-launcher/actions/workflows/tauri.yml/badge.svg)](https://github.com/xpokrzywa/ogulniega-full-launcher/actions/workflows/tauri.yml)
[![Release](https://github.com/xpokrzywa/ogulniega-full-launcher/actions/workflows/release.yml/badge.svg)](https://github.com/xpokrzywa/ogulniega-full-launcher/actions/workflows/release.yml)
[![Strona (GitHub Pages)](https://github.com/xpokrzywa/ogulniega-full-launcher/actions/workflows/pages.yml/badge.svg)](https://github.com/xpokrzywa/ogulniega-full-launcher/actions/workflows/pages.yml)

Faktyczne uruchomienie gry na Androidzie zapewnia zewnętrzny silnik (PojavLauncher i podobne). Ten projekt dostarcza backend pobierania i instalacji w Rust.

## Pobranie

Gotowe paczki (deb, AppImage, APK) pojawiają się przy tagu `v*` na stronie [Releases](https://github.com/xpokrzywa/ogulniega-full-launcher/releases).

## Co robi backend

Komendy Tauri w `src-tauri/src/commands.rs`:

- `list_versions` — lista wersji z `launcher.json` (lustro Ogulniegi),
- `download_file` — pobranie pliku (klient JVM albo jar moda) i zwrot SHA-512 oraz rozmiaru,
- `sha512_of` — suma SHA-512 bez zapisu na dysk,
- `fetch_json` — pobranie JSON,
- `backend_info` — wersja aplikacji, system i architektura (zakładka Start).

## Budowa

Wymagane: Node.js 22, Rust (stable, `rust-version` w `src-tauri/Cargo.toml` to 1.77), dla Androida JDK 21 i Android SDK.

```bash
npm install
npx tauri dev
npx tauri build
npx tauri android build --apk
```

`npx tauri build` składa paczki desktop (w CI: deb i AppImage). APK na Androida buduje job `android` w `release.yml`:

```bash
npx tauri android init --ci   # raz, generuje gen/android (nie jest w gicie)
npx tauri android build --apk --debug
```

## Struktura projektu

```
.
├── www/                  interfejs (HTML/CSS/JS; źródło GitHub Pages)
├── src-tauri/            backend Tauri 2 (Rust): komendy, allowlista, testy
├── src-tauri/src/        commands.rs, download.rs, lib
├── resources/            zasoby aplikacji
└── .github/workflows/    tauri.yml, pages.yml, release.yml
```

Katalog `src-tauri/gen/` generuje `npx tauri android init` i nie jest w gicie (patrz `.gitignore`).

## Bezpieczeństwo

`check_url` w `src-tauri/src/commands.rs` przyjmuje wyłącznie `https` i hosty z `ALLOWED_HOSTS` (16 sztuk): lustro dystrybucji (`cdn.jsdelivr.net`), `ogulniega.com`, hosty Mojanga, Fabric, Modrinth, CurseForge i GitHub. Inne hosty, `http://` i `file://` są odrzucane (`cargo test` to sprawdza). Ten sam `is_allowed_host` sprawdza **każdy hop redirectu** (`Policy::custom` w `download.rs`) — przekierowanie na obcy host jest zatrzymywane.

`check_dest_path` ogranicza zapis do katalogu danych aplikacji: ścieżki względne są do niego doklejane, absolutne tylko jeśli leżą w środku, `..` i `file://` odrzucone w obu przypadkach. `download_file` i `sha512_of` liczą SHA-512 strumieniowo (`sha2::Sha512`); `download_file` zwraca faktyczny `sha512` i `size`, a pobranie idzie do pliku `.part` z atomowym `rename` po zgodności sumy. Limit odczytu JSON (8 MiB, `MAX_JSON_BYTES`) jest sprawdzany **w locie** podczas strumieniowania — odpowiedź nigdy nie ląduje w całości w RAM. Pobieranie: 16 równoległych transferów (semafor), wznawianie przez `Range` i 3 próby z backoffem, `https_only` + timeout 60 s.

Frontend: `esc()` na wszystkich danych z sieci, `safeUrl()` dopuszcza wyłącznie `https://` w `href` (blokada `javascript:`), `rel="noopener noreferrer"` na linkach zewnętrznych. CSP: `default-src 'self'` z zamkniętą listą źródeł (`tauri.conf.json`).

Zgłaszanie podatności: [SECURITY.md](SECURITY.md). Nie commituj tokenów, kluczy API, plików `*.keystore` ani `*.jks`.

## Licencja

[MIT](LICENSE), copyright 2026 xpokrzywa. Projekt nieoficjalny, niepowiązany z autorem klienta Ogulniegi ani z Mojang/Microsoft.
