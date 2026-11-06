# Współtworzenie

Zmiany w backendzie idą przez pull request na `main`. Nie commituj sekretów (tokeny, klucze API, `*.keystore`, `*.jks`).

## Jak zbudować

Node.js 22 i Rust stable (`rust-version` w `src-tauri/Cargo.toml`: 1.77).

```bash
npm install
npx tauri dev
npx tauri build
npx tauri android build --apk
```

Android wymaga JDK 21 i Android SDK (NDK przy budowie Tauri). Pierwszy raz: `npx tauri android init`.

## Sprawdzenia przed PR

Z katalogu `src-tauri`:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI w `.github/workflows/tauri.yml` odpala `cargo test --all-features` i `npx tauri build --bundles deb,appimage`.

## Commity

[Conventional Commits](https://www.conventionalcommits.org/), po polsku albo po angielsku, jeden temat na commit:

```
feat: lista wersji z launcher.json
fix: odrzucanie http:// w check_url
docs: opis allowlisty w README
ci: artefakty deb i apk w release
test: nieznane hosty
```

Typy: `feat`, `fix`, `docs`, `ci`, `test`, `refactor`, `chore`. Breaking change: `feat!:` albo stopka `BREAKING CHANGE:`.
