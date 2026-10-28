# Uwierzytelnione parowanie i szyfrowane Wi-Fi

Kontynuacja zatwierdzonego audytu. Implementacja w bieżącej sesji; niezależny przegląd przed publikacją. Osobna gałąź `codex/secure-pairing`, baza `272097d688680edbec5838642763f4af8b821a4b`, PR do `codex/security-hardening`. Bez scalania i wydania.

## Decyzje

Sterowanie i audio Wi-Fi używają jednego TLS/TCP, bez powrotu do jawnego TCP/UDP. USB zachowuje dotychczasowy protokół wyłącznie na loopback. TLS zapewniają rustls na komputerze i systemowy SSLSocket na Androidzie; Ktor pozostaje adapterem kanałów, bez własnej implementacji kryptografii. Cena tej decyzji: blokowanie TCP przy utracie pakietów wymaga późniejszego pomiaru na telefonie.

Kod `ymic1:<64 małe cyfry hex SHA-256 certyfikatu DER>:<64 cyfry hex losowego sekretu>` przekazuje się poza sieciowym protokołem połączenia, z lokalnego UI komputera do ustawień telefonu. To współdzielone hasło, nie identyfikator urządzenia. Odcisk przypina tożsamość serwera niezależnie od zmiany IP. Wygenerowanie nowego kodu po zatrzymaniu serwera unieważnia wszystkie wcześniejsze kopie, nie pojedynczy telefon.

Po TLS telefon wysyła 8 bajtów `YMICv1\0\0` i 32 bajty sekretu. Serwer porównuje sekret w stałym czasie i odpowiada `YMOK`, dopiero potem zaczyna dotychczasowy handshake MicYou i protobuf. Nieautoryzowany klient nie otrzymuje sesji ani dostępu do audio/sterowania. TLS i autoryzacja mają limity czasu i równoległych prób. Wersja bez parowania nie jest akceptowana przez Wi-Fi.

Tożsamość (certyfikat, klucz i token) jest trwale zapisana per użytkownik. Windows: DPAPI CurrentUser bez interakcji. Unix: prywatny katalog 0700 i plik 0600. Android: AES/GCM z kluczem Android Keystore, zapis w noBackupFilesDir; nie ujawniać kodu w logach ani zwykłych preferencjach. Wi-Fi w wariancie Android API 21/22 jest niedostępne, ponieważ wybrana ochrona klucza wymaga API 23; USB pozostaje dostępne.

Rotację i nasłuch Wi-Fi synchronizuje rzeczywista blokada pliku systemu operacyjnego (Rust File::try_lock/try_lock_shared, Rust 1.89+). Nasłuch utrzymuje blokadę współdzieloną do zakończenia wszystkich klientów, rotacja wymaga wyłącznej. Plik PID nie jest autoryzacją ani rezerwacją; awaria procesu automatycznie zwalnia blokadę OS. GUI dopuszcza odnowienie również na już zatrzymanym serwerze i po uszkodzeniu starego zapisu.

Decyzja po przeglądzie: tryb web zostaje wyłączony na granicy backendu i w UI. Strona podmieniona przez MITM mogłaby wykraść sekret z fragmentu po zaakceptowaniu fałszywego certyfikatu. Nie udostępniać wspólnego tokenu w przeglądarce. Przywrócenie web wymaga osobnego modelu zaufania certyfikatu i oddzielnych uprawnień. Natywny Android sprawdza pin cert DER przed wysłaniem tokenu.

## Zadania

- [ ] `transport/secure.rs`: generowanie, bezpieczny zapis/odczyt, eksport i rotacja tożsamości; stałoczasowa weryfikacja tokenu; testy limitów pliku, uprawnień, odtwarzania i unieważnienia.
- [ ] `tcp.rs`, `server/service.rs`: TLS i autoryzacja przed przyjęciem Wi-Fi; wspólny generyczny handler streamu dla TLS i USB; brak nasłuchu UDP dla Wi-Fi; testy prawdziwego TLS i niewłaściwego tokenu/certyfikatu/jawnego klienta. Testy starego protokołu przenieść na USB.
- [ ] Android `network/Pairing*.kt`: parser kodu, Keystore, SSLSocket z odciskiem i adapter Ktor Socket zachowujący istniejącą obsługę zatrzymywania. AudioEngine wymusza TLS/TCP w Wi-Fi i wysyła autoryzację przed handshake; brak nagrywania/streamingu po błędnym pinie lub tokenie.
- [ ] Lokalne UI: panel kodu/odnowienia w ustawieniach desktopu, komenda CLI pair, pole zapisu/zapomnienia parowania w Androidzie. Napisy we wszystkich plikach lokalizacji, domyślnie po angielsku. Nigdy nie logować kodu ani wyświetlać go automatycznie.
- [ ] WebSocket: blokada WebServer::start oraz trybu web w UI. Nie wystawiać tokenu przez HTTP, URL ani QR przeglądarki. Test odmowy przed bind/generowaniem certyfikatu.
- [ ] Testy integracyjne Rust/TLS, testy parserów i UI, typecheck, CI Rust/Clippy, Windows i Android. Lokalnie nie instalować Rust/JDK ani sterowników, nie zmieniać ExecutionPolicy. Test telefonu nadal wymaga realnego urządzenia; nie traktować buildów jako dowodu opóźnienia lub poprawnego przechwytywania mikrofonu.

## Granice

Chronimy przed niesparowanym klientem i podsłuchem sieci, nie przed złośliwym procesem działającym jako ten sam użytkownik ani ujawnieniem kodu przez właściciela. Wciąż wymagają pracy: szybki start Androida, aktualizatory, zależności wskazane audytem, podpisy i pochodzenie binariów. Nie deklarować pełnego bezpieczeństwa produktu.
