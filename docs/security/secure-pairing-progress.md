# Parowanie i szyfrowane Wi-Fi — stan weryfikacji

Baza: `272097d688680edbec5838642763f4af8b821a4b` z PR #1. Nowa gałąź: `codex/secure-pairing`. Plan: docs/superpowers/plans/2026-10-28-secure-pairing.md.

Implementacja wymusza TLS 1.2/1.3 oraz sekret parowania przed handshake MicYou w trybie Wi-Fi. Android sprawdza SHA-256 certyfikatu leaf DER przed wysłaniem sekretu. Sterowanie i audio przechodzą przez ten sam TLS/TCP; Wi-Fi nie uruchamia jawnego UDP ani fallbacku. USB zachowuje lokalny protokół loopback. Obie aplikacje muszą zostać zaktualizowane.

Kod parowania jest hasłem współdzielonym przez zaufane telefony. Jest ujawniany tylko na żądanie lokalnego użytkownika przez panel ustawień lub `micyou-cli pair`; nie przez mDNS, HTTP ani logi. `micyou-cli pair --reset` lub odnowienie w GUI unieważnia wszystkie wcześniejsze parowania. Windows przechowuje tożsamość przez DPAPI CurrentUser, Unix w prywatnym katalogu/pliku, Android przez Keystore AES/GCM bez kopii zapasowej. Odcisk nie zmienia się przy zmianie IP. Uszkodzony zapis wymaga jawnego resetu, bez cichej regeneracji.

Przegląd wykrył dwa problemy rotacji: odmowę resetu na już zatrzymanym serwerze oraz wyścig między sprawdzeniem pliku PID a startem innego procesu. Oba poprawiono. Rzeczywista blokada OS współdzielona jest trzymana przez nasłuch Wi-Fi do zakończenia klientów; rotacja wymaga wyłącznej blokady. Nie polega na pliku PID i zwalnia się po awarii procesu.

Tryb web został wyłączony w backendzie i selektorach. Nie przekazujemy mu wspólnego sekretu: strona podmieniona po zaakceptowaniu fałszywego certyfikatu mogłaby go wykraść. Przywrócenie web wymaga osobnego modelu zaufania.

Lokalnie: kontrola typów Vue przeszła, 9 testów Bun przeszło, 1 test zachowania PowerShell pominięto zgodnie z polityką hosta. Nie zmieniano ExecutionPolicy ani nie instalowano lokalnie Rust/JDK. `git diff --check` przeszedł.

Nowe testy Rust obejmują ochronę zapisu, trwałość kodu, odmowę przy uszkodzonym zapisie, rotację i blokady OS, prawdziwy TLS z dźwiękiem/sterowaniem oraz odrzucenie błędnego tokenu, jawnego klienta i błędnego certyfikatu. Testy JVM generują wyłącznie efemeryczny certyfikat testowy przez JDK keytool i sprawdzają systemowy SSLSocket, pin przed danymi aplikacji oraz przerwanie blokującego handshake. Ich kompilację i wyniki musi potwierdzić CI; nie były uruchamiane lokalnie. Nowy obowiązkowy job Android nie ma continue-on-error.

Nie dopuszczać tej gałęzi do wydania przed CI i próbą telefonu. Brakuje testów Keystore/DPAPI po utracie profilu, scenariusza blokady między osobnymi procesami oraz pomiarów opóźnienia i zatrzymywania na Androidzie. Pozostają też zadania pierwotnego audytu: szybki start Androida, aktualizatory, zależności i pochodzenie/podpisy binariów. To nie jest deklaracja pełnego bezpieczeństwa produktu.
