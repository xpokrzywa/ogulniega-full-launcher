# Ogulniega Full Launcher

**Pełny launcher Minecraft (Java / Fabric) w Rust dla Android — nowy projekt.**

Celem projektu jest: użytkownik wybiera wersję z listy Ogulniegi → klik „Graj” →
aplikacja pobiera klienta JVM i mody (z weryfikacją SHA-512) → dostarcza do silnika.

> **Uwaga:** faktycznego uruchomienia gry na Androidzie zapewnia zewnętrzny silnik
> (PojavLacher i podobne). Ten projekt dostarcza backend pobierania i instalacji w Rust.

## Co robi backend

- `list_versions` — lista wersji z `launcher.json` (lustro Ogulniegi),
- `download_file` — pobór pliku (klient JVM / mod jar) z SHA-512 + rozmiar,
- `sha512_of` — weryfikacja sumy bez zapisu,
- `fetch_json` — pobór JSON.

## Szybki start

```bash
npm install
npx cap sync android   # lub: npx tauri android init
cd android && ./gradlew assembleDebug
```

## Budowa

- desktop: `npx tauri build` (deb/AppImage)
- android: `npx tauri android build --apk --debug`

## Licencja

MIT — patrz [LICENSE](LICENSE). Projekt nieoficjalny, niepowiązany z autorem klienta
Ogulniegi ani z Mojang/Microsoft.
