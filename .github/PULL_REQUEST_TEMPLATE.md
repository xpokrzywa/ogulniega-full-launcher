## Co zmienia ten PR

-

## Jak sprawdzić

- [ ] `cargo fmt --check` (katalog `src-tauri`)
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] budowa, jeśli rusza UI albo pakowanie: `npx tauri build` albo `npx tauri android build --apk`

## Uwagi

Allowlista hostów albo weryfikacja SHA-512: opisz zmianę. Bez tokenów i kluczy API w diffie.
