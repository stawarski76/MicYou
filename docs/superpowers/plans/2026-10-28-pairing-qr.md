# Pairing QR Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Sparować telefon przez zeskanowanie kodu QR z aplikacji Windows, bez przepisywania sekretu.

**Architecture:** Desktop koduje istniejący ciąg `ymic1:<pin>:<token>` lokalnie za pomocą obecnej zależności qrcode. Android uruchamia własny, niewyeksportowany ekran skanera ZXing Embedded, akceptuje wyłącznie QR z tym formatem i zapisuje go istniejącym PairingStore. Protokół TLS i przechowywanie poświadczeń pozostają wspólne dla obu sposobów wprowadzania.

**Tech Stack:** Vue 3, qrcode 1.5.4 (już w projekcie), Kotlin/Compose, ZXing Android Embedded 4.3.0, ZXing core.

**Spec:** Bezpośrednia prośba użytkownika z tej rozmowy: QR w aplikacji desktopowej i skanowanie aplikacją Android zamiast ręcznego wpisywania.

## Global Constraints

- Sekret i obraz QR nie trafiają do sieci, logów ani trwałego magazynu UI desktopu.
- QR jest widoczny tylko na żądanie i znika po ukryciu, błędzie lub odnowieniu. Asynchroniczne generowanie nie może przywrócić starego sekretu.
- Skaner nie otwiera URL, nie zapisuje zdjęć, uruchamia kamerę tylko po naciśnięciu przycisku i jest android:exported=false.
- Ręczne wprowadzenie pozostaje opcją; anulowanie skanowania nie usuwa poprzedniego parowania.
- Android zapisuje poprawny wynik przez Keystore/PairingStore; błędny QR nie zmienia magazynu. Brak automatycznego uruchamiania mikrofonu.
- Nie instalujemy lokalnie Rust/JDK. Kompilacja i testy Androida przez CI. Istniejący wydzielony checkout pozostaje używany, bez resetowania zmian.

## Review Focus

- Ukrycie/rotacja podczas generowania QR nie ujawnia poprzedniego obrazu.
- Błąd generowania nie ujawnia błędu backendu ani sekretu.
- Anulowanie, odmowa kamery i inny QR nie nadpisują poprawnego parowania.
- QR ma czarny kod na białym tle i cztery moduły marginesu również w ciemnym motywie.
- Telefon odkodowuje dokładnie format wygenerowany przez bibliotekę desktopu.

## Task 1: QR na desktopie

Files: PairingPanel.vue, nowy utils/pairingQr.ts, tests/security/pairing-panel.test.ts, wszystkie desktopowe locales.

- [ ] Dodać test renderowania QR dopiero po odsłonięciu i usuwania po ukryciu/błędzie; zobaczyć niepowodzenie przed implementacją.
- [ ] Generować PNG data URL lokalnie z aktualnego kodu; zabezpieczyć asynchroniczne wyniki numerem żądania i wyczyścić poprzedni sekret przed rotacją.
- [ ] Sprawdzić testy Bun i vue-tsc; wygenerować efemeryczny fixture QR z fałszywym kodem do testu między platformami.

## Task 2: Skanowanie na Androidzie

Files: build.gradle.kts, AndroidManifest.xml, PairingSetupCard.kt, nowy PairingCaptureActivity.kt, test PairingQrTest.kt i test resource, wszystkie Androidowe strings.xml.

- [ ] Dodać test odkodowania fixture desktopu i sprawdzenia PairingCredentials.
- [ ] Przypiąć bibliotekę skanera; dodać niewymaganą kamerę i niewyeksportowany ekran z FLAG_SECURE.
- [ ] Dodać ScanContract z filtrem QR, bez beep/zdjęć, timeoutem i lokalizowanymi komunikatami. Po poprawnej walidacji zapisać w PairingStore; nie uruchamiać audio.
- [ ] Sprawdzić uprawnienia, anulowanie, lifecycle i zgodność z istniejącym api21 desugaring.

## Task 3: Weryfikacja i pakiety

- [ ] Wykonać jeden przegląd całej zmiany i poprawić konkretne błędy.
- [ ] Opublikować na gałęzi PR #2, uruchomić CI frontend/JVM/APK/Windows i dostarczyć nowe pakiety po udanych buildach.
- [ ] Odnotować brak fizycznego testu aparatu i skanowania na telefonie. Bez merge/release.
