# Ochrona aktywnej sesji mikrofonu

> Implementacja w bieżącej sesji przez superpowers:executing-plans. Kontynuacja zatwierdzonego audytu i poprawek; bez scalania i wydawania aplikacji.

**Cel:** drugi klient TCP lub WebSocket nie może automatycznie wyprzeć aktywnego nadawcy ani zmienić jego stanu audio.

**Architektura:** zachować protokół Androida i dotychczasowe biblioteki. TCP serializuje przyjęcie i zakończenie sesji jednym istniejącym zamkiem. WebSocket atomowo rezerwuje jedno miejsce nadawcy; zajęte miejsce powoduje zamknięcie nowego połączenia bez zdarzeń i zmian licznika. Origin musi być pojedynczy, poprawny, HTTPS i zgodny z Host oraz lokalnym adresem serwera.

**Podstawa:** SEC-04/06 w docs/security/2026-10-27-micyou-audit.md. To ograniczenie przejmowania, a nie uwierzytelnienie. Pierwszy niesparowany klient nadal może zająć wolny serwer. Natywne TCP/UDP pozostają nieszyfrowane. Native WebSocket może sfałszować Origin.

## Zadania i warunki testów

- [ ] TCP: przed rezerwacją kolejki audio sprawdzić wolne miejsce pod takeover_lock. Odrzucić zajęte miejsce bez zmiany tokenu, epoch, kanału pluginów lub zdarzeń. Sprawdzić anulowanie przed publikacją. Zakończenie sesji obejmuje tym samym zamkiem wyczyszczenie połączenia, audio i kanału pluginów.
- [ ] TCP: test przez rzeczywiste sockety loopback — pierwszy Connect, drugi Connect odrzucony; token i audio pierwszego zachowane, bez dodatkowego SessionStarting. Pierwszy nadal odpowiada na ping. Po jego zakończeniu trzeci klient zostaje przyjęty.
- [ ] WebSocket: zastąpić wymianę nadawcy odmową przy zajętym miejscu. Testy: odmowa bez anulowania pierwszego, zwolnienie i ponowne przyjęcie, brak zwolnienia przez stary generation, anulowany serwer nie przyjmuje, równoległe próby mają jednego zwycięzcę.
- [ ] Origin: parsować przy pomocy istniejącego axum::http, wymagać jednego Origin i Host, HTTPS, tej samej pary host/port, lokalnego hosta. Testy obejmują brak/duplikaty, HTTP, inne porty/hosty, userinfo, ścieżki/query/fragmenty i IPv6.
- [ ] Uruchomić dostępne lokalne kontrole, niezależny przegląd kodu, opublikować commit do draft PR #1 i potwierdzić testy Rust/Clippy oraz build Windows w CI. Nie instalować lokalnie Rust, nie zmieniać zapory ani sterowników.

## Skutki użytkowe

Zmiana telefonu wymaga zakończenia pierwszego połączenia albo zatrzymania/uruchomienia serwera. Nagłe zerwanie sieci może blokować nowe połączenie do istniejącego timeoutu 10 sekund. Web mode nadal korzysta z HTTPS; klient bez Origin lub z innej strony/portu jest odrzucany.

## Dalszy etap

Pełne rozwiązanie SEC-04/05/06 wymaga wersjonowanego, uwierzytelnionego i szyfrowanego transportu po obu stronach, jawnego parowania i testów rzeczywistych urządzeń. Nie dodawać własnych algorytmów kryptograficznych ani traktować kontroli Origin/IP jako tożsamości urządzenia. Osobno sprawdzić przypinanie tożsamości serwera, ochronę sekretów, cofanie parowania i brak cichego przejścia do transportu jawnego.
