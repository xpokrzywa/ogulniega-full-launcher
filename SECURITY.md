# Zgłaszanie podatności

Nie otwieraj publicznego issue ani pull requesta z opisem exploita, tokenem albo kluczem.

Napisz do opiekuna repozytorium (xpokrzywa) przez [prywatne zgłoszenie podatności na GitHub](https://github.com/xpokrzywa/ogulniega-full-launcher/security/advisories/new) albo e-mail podany w profilu GitHub. Podaj:

- wersję albo commit,
- kroki odtworzenia,
- czego dotyczy (pobieranie, allowlista hostów, suma SHA-512, podpis APK).

Odpowiedź w ciągu 7 dni. Poprawka albo uzasadnione odrzucenie w ciągu 30 dni, o ile da się to zrobić bez zmiany zewnętrznego silnika gry.

## Czego nie wrzucać do repozytorium

- tokenów (GitHub, npm, CI), haseł i kluczy API,
- plików `*.keystore`, `*.jks` i haseł do podpisu Androida,
- `local.properties` i prywatnych adresów z sekretem w query stringu,
- dumpów kont Minecraft i ciasteczek sesji.

Allowlista pobierania jest w `ALLOWED_HOSTS` w `src-tauri/src/commands.rs`. Nowe hosty dodawaj świadomie: `check_url` puszcza każdy URL zaczynający się od dopisanego prefiksu.
