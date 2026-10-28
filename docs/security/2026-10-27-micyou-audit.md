# Audyt bezpieczeństwa MicYou przed utworzeniem Yelset Mic

Data: 27 października 2026. Status: **nie dopuszczać tej wersji do wydania jako bezpieczny produkt**.

## Materiał i granice oceny

- Źródło: https://github.com/MicYou-Dev/MicYou, tag `v2.1.1`.
- Dokładny commit: `d821da032e2b72330ced09678dd127acd2714bef`.
- Lokalna gałąź: `codex/micyou-security-audit`.
- Przegląd statyczny transportu, Androida, pluginów, interfejsu Tauri, instalatora Windows i pobierania plików; dwa niezależne przeglądy transportu i pluginów.
- Zapytania OSV o wersje z Cargo.lock i bun.lock, bez instalowania lub wykonywania zależności.
- Nie uruchomiono aplikacji, instalatora, sterownika, skryptów budowania ani złośliwych pluginów. Nie wykonano prób ataku na inne urządzenia.
- Nie zweryfikowano zgodności binariów wydań ze źródłami, podpisów instalatorów, wnętrza VB-CABLE ani plików natywnych/modeli dołączonych do repozytorium.
- Brak kompilacji, testów Rust/Android, fuzzingu i prób WebView/urządzeń. Cargo, Rust i JDK nie zostały znalezione w PATH tej sesji. Nie oznacza to dowodu braku ich instalacji na całym komputerze.

Wnioski dotyczą tego commitu, nie wszystkich wersji MicYou. Nie ustalono obecności wirusa lub celowego backdoora. Statyczny przegląd nie stanowi certyfikatu bezpieczeństwa.

## Potwierdzone problemy w źródłach

### SEC-01 — wysokie: opis pluginu może wstrzyknąć JavaScript do głównego okna

Miejsca: `tauri-app/src/features/plugins/components/PluginMarketDialog.vue:312`, `:516`, `:551`, `:582`, `:595`; `tauri-app/src-tauri/tauri.conf.json:34`.

Zewnętrzny README przechodzi filtr HTML **przed** zamianą Markdown na HTML. Zamiana obrazków wkłada nieescaped tekst do atrybutu `alt`, a wynik trafia do `v-html`. Niewinny demonstrator źródłowy:

```text
!["onerror="alert(1)](x)
```

Po zamianie obrazka powstaje atrybut `alt=""onerror="alert(1)"`. Brak odstępu przed atrybutem nie jest zabezpieczeniem parsera HTML. Filtr nie jest stosowany ponownie, a CSP w Tauri jest wyłączone.

Warunek: napastnik kontroluje README pluginu dostępnego z marketu, a użytkownik otwiera opis. Nie wymaga zatwierdzenia instalacji. Wstrzyknięty skrypt działa w uprzywilejowanym oknie aplikacji; wywołanie komend instalacji natywnego pluginu daje potencjalną drogę do wykonania kodu na komputerze (`tauri-app/src-tauri/src/app.rs:140`, `tauri-app/crates/micyou-core/src/plugins/install.rs:450`, `:515`).

Poziom dowodu: analiza transformacji w źródłach; wykonanie w WebView i pełny łańcuch do kodu natywnego wymagają izolowanego testu dynamicznego.

Naprawa: wyłączyć market do czasu poprawek; użyć utrzymywanego renderera Markdown i sanitizera końcowego HTML, walidować schematy URL, przywrócić restrykcyjny CSP. Zgoda na instalację musi być sprawdzana również w backendzie.

### SEC-02 — wysokie: plugin WASM z fs.write może zmienić się w natywny plugin

Miejsca: `tauri-app/crates/micyou-core/src/plugins/api.rs:252`, `tauri-app/crates/micyou-plugin/src/manager.rs:126`, `tauri-app/crates/micyou-core/src/plugins/mod.rs:141`, `:215`, `tauri-app/crates/micyou-plugin/src/native.rs:91`.

Zapisy są ograniczone do katalogu pluginu, ale obejmują również jego `plugin.json`. Zatwierdzony plugin WASM z `fs.write` może zawierać nieaktywną DLL i zmienić manifest, wybierając runtime natywny. Przy ponownym uruchomieniu manifest jest czytany z dysku, a zapisany stan enabled powoduje automatyczne załadowanie nowego runtime bez ponownej zgody. Podobna zmiana może rozszerzyć deklarowane capabilities.

Warunek: użytkownik wcześniej zatwierdził plugin WASM z zapisem; później uruchomił aplikację ponownie. Skutek: ominięcie granicy WASM i wykonanie kodu natywnego z prawami użytkownika.

Naprawa: osobny zapisywalny katalog danych; manifest i kod poza nim; niezależnie zapisana zgoda obejmująca hash, runtime i capabilities. Zmiana wymaga ponownego zatwierdzenia. Jawnie zaakceptowany natywny plugin z założenia ma pełny dostęp — sam ten model zaufania nie jest luką.

### SEC-03 — wysokie: zgoda na uprawnienia nie jest związana z pobranym archiwum

Miejsca: `tauri-app/src/features/plugins/components/PluginMarketDialog.vue:468`, `:482`; `tauri-app/crates/micyou-core/src/plugins/install.rs:485`, `:509`, `:515`.

Podgląd uprawnień pochodzi z manifestUrl, a instalacja niezależnie pobiera downloadUrl. Backend nie otrzymuje zaakceptowanego zestawu uprawnień, runtime lub skrótu. Instaluje identyfikator z archiwum i automatycznie uruchamia plugin. Złośliwy wydawca lub przejęty endpoint paczki może pokazać łagodny opis i dostarczyć kod natywny z innymi uprawnieniami.

Naprawa: najpierw pobrać i sprawdzić konkretną paczkę, następnie potwierdzać jej hash, ID, runtime i uprawnienia; wymuszać zgodność w backendzie. Tak samo traktować aktualizacje pluginów.

### SEC-04 — wysokie: dowolny osiągalny klient TCP może przejąć mikrofon

Miejsca: `tauri-app/crates/micyou-core/src/transport/tcp.rs:477`, `:500`, `:547`; `tauri-app/crates/micyou-protocol/src/lib.rs:33`.

Powitanie opiera się na publicznych stałych MicYouCheck1/MicYouCheck2. Nie uwierzytelnia urządzenia. Po odczytaniu pierwszej poprawnej ramki nowa sesja zastępuje dotychczasową i zamyka połączenie telefonu. Identyfikator sesji jest dostarczany przez klienta; kontrola IP/ID w UDP nie zastępuje uwierzytelnienia.

Warunek: uruchomiony, osiągalny port serwera. Skutek: rozłączenie telefonu, podstawienie własnego dźwięku i zaakceptowanych komunikatów sterujących.

Naprawa: uwierzytelnione parowanie przed zmianą aktywnej sesji, klucze przypisane do sparowanego urządzenia, kontrolowane przejmowanie połączenia.

### SEC-05 — wysokie: natywna transmisja Wi-Fi nie chroni poufności głosu

Miejsca: `composeApp/src/main/kotlin/com/lanrhyme/micyou/audio/AudioEngine.kt:624`, `:672`, `:742`, `:1202`.

TCP to zwykły socket; UDP wysyła nagłówek i serializowane audio bez szyfrowania. Brakuje również kryptograficznego potwierdzenia tożsamości komputera.

Warunek: dostęp do obserwowania trasy transmisji, np. przejęty router/AP. Samo bycie w tej samej nowoczesnej sieci Wi-Fi nie gwarantuje możliwości podsłuchu. Skutek: odczyt głosu; aktywny napastnik na trasie może podszyć się pod odbiornik.

Naprawa: uwierzytelniony szyfrowany transport obejmujący sterowanie i audio; TLS dla sterowania i AEAD dla datagramów albo wspólny bezpieczny transport. Nie projektować własnych algorytmów kryptograficznych.

### SEC-06 — wysokie: WebSocket z TLS nie wymaga sparowania nadawcy

Miejsca: `tauri-app/crates/micyou-core/src/transport/web.rs:367`, `:388`, `:410`, `:508`, `:658`.

Brak Origin jest akceptowany; nie ma tokenu parowania ani innego uwierzytelniania nadawcy. Natywny klient WebSocket może pominąć Origin i po połączeniu zastąpić aktualnego nadawcę. TLS szyfruje transmisję, ale nie upoważnia klienta do wysyłania dźwięku.

Warunek: włączony tryb web i osiągalny port HTTPS. Skutek: wstrzyknięcie audio i rozłączenie prawidłowego telefonu. Ta ścieżka przyjmuje audio od przeglądarki; nie ustalono możliwości podsłuchu istniejącego strumienia przez samo połączenie do WebSocket.

Naprawa: token/sparowana tożsamość sprawdzane przed upgrade i aktywacją; Origin pozostawić jako dodatkową ochronę.

### SEC-07 — średnie: reguły zapory obejmują również sieci publiczne

Miejsce: `tauri-app/src-tauri/installer.iss:123` (również :124 i :125).

Domyślnie zaznaczana instalacja reguł dodaje allow dla trzech programów, profile=any, bez ograniczenia do portu i LocalSubnet. Nie otwiera wszystkich programów i nie omija routera NAT, lecz niepotrzebnie zwiększa osiągalność nieautoryzowanego serwera w publicznych sieciach.

Naprawa: profil Private, konkretne porty i ograniczenie adresów do oczekiwanej sieci; użytkownik powinien świadomie rozszerzyć zakres, jeśli tego potrzebuje.

### SEC-08 — średnie: pobrany instalator sterownika nie jest weryfikowany przez aplikację

Miejsca: `tauri-app/crates/micyou-core/src/platform/vbcable.rs:120`, `:135`, `:159`, `:170`, `:181`.

ZIP jest pobierany z HTTPS producenta, rozpakowywany do przewidywalnego katalogu tymczasowego, a znaleziony EXE uruchamiany przez UAC. Aplikacja nie sprawdza oczekiwanego hasha ani wydawcy Authenticode. Warunki ataku obejmują przejęte źródło paczki lub podmianę zapisywalnego pliku przed uruchomieniem; sama obecność HTTPS nie zapewnia autentyczności oczekiwanego artefaktu w takich przypadkach. UAC nadal wymaga zgody; nie stwierdzono jego obejścia.

Naprawa: sprawdzenie podpisu i oczekiwanego wydawcy, rozmiaru/statusu pobrania i wersji paczki; prywatny nieprzewidywalny katalog; zabezpieczenie przed podmianą po sprawdzeniu. Alternatywnie ręczna instalacja ze strony producenta do czasu wdrożenia poprawnej weryfikacji.

### SEC-09 — średnie, do próby na urządzeniu: zewnętrzna aplikacja może wywołać szybki start mikrofonu Androida

Miejsca: `composeApp/src/main/AndroidManifest.xml:40`; `composeApp/src/main/kotlin/com/lanrhyme/micyou/MainActivity.kt:148`, `:189`, `:271`.

Eksportowana aktywność launchera ufa publicznej akcji ACTION_QUICK_START i może uruchomić stream do zapisanego odbiornika. Inna aplikacja może wysłać jawny Intent bez własnego RECORD_AUDIO. MicYou musi już mieć zgody i poprawny osiągalny adres. Wskaźnik mikrofonu i powiadomienie nadal są widoczne. Ograniczenia nowszych Androidów mogą blokować nagrywanie po schowaniu aktywności; brak testu urządzenia nie pozwala potwierdzić wszystkich wersji OS.

Naprawa: obsługa szybkiego startu przez nieeksportowaną aktywność/PendingIntent; launcher nie może ufać samemu stringowi akcji.

## Zależności — wyniki OSV wymagające analizy osiągalności

Źródła i pełne odpowiedzi: `dependencies-osv.json`; narzędzie `scan-lockfiles.mjs` nie wykonuje kodu pakietów. Sprawdzono 800 unikalnych współrzędnych crates.io i 214 npm. OSV wskazało 13 par pakiet/wersja (łącznie 26 rekordów ostrzeżeń). To nie jest liczba odrębnych podatności: są aliasy GHSA/RUSTSEC, ostrzeżenia o braku utrzymania oraz zależności tylko innych platform lub narzędzi budowania. Parser inwentaryzacji jest dostosowany do obecnego formatu lockfile; po zmianie formatu należy sprawdzić, czy nie pomija wpisów.

Najważniejsze do weryfikacji i aktualizacji:

| Pakiet z lockfile | Wersja | Wynik / wskazana poprawka |
| --- | --- | --- |
| rustls | 0.23.45 | RUSTSEC-2026-0346/0347/0348/0349; poprawka 0.23.46 |
| rustls-webpki | 0.103.15 | RUSTSEC-2026-0382; poprawka w gałęzi 0.103: 0.103.16 |
| webpki-root-certs | 1.0.9 | RUSTSEC-2026-0383; poprawka 1.0.10 |
| zip | 8.6.0 | RUSTSEC-2026-0367/0368/0369; poprawka 9.0.0; opisane efekty Unix nie oznaczają automatycznie exploita Windows |
| tokio | 1.53.1 | RUSTSEC-2026-0386; poprawka 1.53.3; AsyncFd wymaga oceny platformy i faktycznej ścieżki |
| vite | 8.3.2 | Sześć dopasowań GHSA; narzędzie developerskie, nie serwer spakowanej aplikacji; m.in. GHSA-9jrq-w75r-8gcw wskazuje 8.3.3 |
| glib | 0.18.5 | RUSTSEC-2024-0429 / alias GHSA; stos Linux, nie dowód problemu w wydaniu Windows |
| atty, proc-macro-error, zbus_names, zvariant, zvariant_derive | różne | Część wpisów to unmaintained; atty ma również ostrzeżenie o wyrównaniu odczytu |
| block-buffer | 0.10.4 | OSV zwróciło GHSA-qwgh-2vcv-g2f7 dla nazwy block_buffer; rozbieżność nazwy wymaga wyjaśnienia, nie klasyfikować bezwarunkowo jako potwierdzonej podatności tej zależności |

Źródła: https://api.osv.dev/v1/querybatch i poszczególne https://osv.dev/vulnerability/ID. Nie sprawdzono pełnego grafu Gradle, gitowego Rusopus, narzędzi CI, zasobów binarnych i WebView2 w systemie. Cargo.lock przypina Rusopus do commitu; manifest wybiera master, więc odtwarzalność wymaga zachowania --locked.

## Dalsze obserwacje i istniejące zabezpieczenia

- Pluginowe archiwa nie mają limitu liczby plików/łącznego rozmiaru po rozpakowaniu; odczyt manifestu nie ma ścisłego limitu. Możliwy DoS po wybraniu wrogiej paczki.
- Wymagają testu: kolizja wielu plugin.json w ZIP i uruchamianie dużej liczby timerów z WASM. Nie przedstawiać tych kandydatów jako przetestowanych exploitów.
- Aktualizacje APK pobierają plik bez sprawdzenia oczekiwanego hasha/tożsamości pakietu przez aplikację. Android nadal kontroluje instalację i zgodność podpisu istniejącego pakietu; nie ustalono cichej podmiany aplikacji.
- TCP ma limit ramki 1 MiB, timeouty i limit 64 klientów. UDP sprawdza format, długości, IP i sesję. USB jest ograniczone do loopback. Web ma TLS i limity zasobów.
- Plugin ZIP używa enclosed_name i ręcznie tworzy zwykłe pliki; nie ustalono bezpośredniego Zip Slip w tej konkretnej ścieżce. Nie mylić jej z ZipArchive::extract w integracji sterownika.
- WASM ma budżet paliwa, a plugin panels są w sandboxed iframe z kontrolą źródła wiadomości. To nie usuwa opisanych wyżej obejść granic.
- Workflow AI review używa zewnętrznej akcji @latest i zapisuje komentarze; przed włączeniem Actions w forku należy przeglądnąć i przypiąć akcje, usunąć niepotrzebne integracje oraz zredukować uprawnienia. Nie skopiowano sekretów upstream.

## Zalecana kolejność dopuszczenia do wydania

1. Fork na GitHubie z zachowaniem licencji i historii; przypiąć bazę audytu do powyższego commitu. Nie uruchamiać workflow bez przeglądu.
2. Domyślnie wyłączyć market/pluginy i zewnętrzne aktualizatory do czasu usunięcia SEC-01/02/03. To propozycja następnego etapu, nie wykonana poprawka.
3. Naprawić parowanie, uwierzytelnianie, szyfrowanie i reguły zapory (SEC-04/05/06/07).
4. Zweryfikować pobierane pliki i zmienić szybki start Androida (SEC-08/09).
5. Zaktualizować zależności po analizie zgodności i osiągalności; odtworzyć graf Gradle; wdrożyć regularne skany.
6. Uruchomić testy regresji, izolowane testy WebView/pluginów, test dwóch niesparowanych klientów i próby na rzeczywistym telefonie.
7. Zbudować własne artefakty, sprawdzić podpisy, przeskanować binaria i udokumentować pochodzenie. Dopiero później polski instalator i test użytkowy.

## Stan wykonania i późniejsze zmiany

Powyższy raport opisuje stan historycznej bazy v2.1.1, przed poprawkami. Użytkownik następnie utworzył fork https://github.com/stawarski76/MicYou. Pierwszy patch na gałęzi `codex/security-hardening` bazuje na jego master `692ecbb9cee1ac45da590cadb59b645f23c1ea65`.

Patch blokuje zewnętrzne pluginy w GUI, CLI i backendzie, usuwa interpretowanie README jako HTML oraz ogranicza reguły zapory. Jest to ograniczenie powierzchni ataku, nie pełne rozwiązanie modelu uprawnień pluginów. Bieżące wyniki testów i nierozwiązane ryzyka opisano w [hardening-progress.md](hardening-progress.md). Zalecenia powyżej pozostają listą warunków wydania; nie oznaczają wykonanych testów ani potwierdzenia bezpieczeństwa aplikacji.
