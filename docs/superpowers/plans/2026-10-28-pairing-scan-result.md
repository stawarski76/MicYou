# Pairing scan result repair

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wynik skanowania nie ginie przy odtworzeniu listy ustawień i zawsze ma czytelny rezultat.

**Architecture:** Launchery aparatu/skanera są rejestrowane stale w MainActivity. Wynik waliduje i zapisuje sesja należąca do Activity ViewModel, niezależna od obecności karty Compose. UI obserwuje wynik, a aktywność wyświetla krótki komunikat. Sekret nie trafia do rememberSaveable, logów ani komunikatów. Sam skan nie rozpoczyna nagrywania.

**Evidence:** Użytkownik zgłasza zamknięcie skanera bez komunikatu. Obecny callback w LazyColumn pomija null/disabled bez feedbacku, a ustawienia są tylko remember, więc odtworzenie aktywności może zgubić ekran/obsługę. Brak logów urządzenia: to naprawa wykazanych słabych punktów, nie potwierdzenie konkretnego przebiegu na telefonie.

- [ ] Testy: sukces dopiero po zapisie bez obserwatora UI, błędny format/QR, anulowanie, aktywne audio, błąd magazynu.
- [ ] Sesja wyniku i Activity ViewModel; stałe launchery MainActivity i feedback bez sekretów.
- [ ] Karta obserwuje wynik; przywracanie ekranu ustawień przez rememberSaveable i stały klucz karty.
- [ ] CI JVM/APK i dostarczenie poprawionego APK z QR do pobrania. Desktop bez zmian, bez merge/release.
