# Scenariusze API P0 emitenta

Skrypt `scripts/run-p0-api-tests.py` wykonuje na uruchomionym wdrożeniu
przykładowe przebiegi EM-01–EM-05. Łączy się z API emitenta na porcie `3000`
oraz mockBankiem na porcie `3100`.

Zakres obejmuje utworzenie i idempotentne odtworzenie zlecenia emisji,
odrzucenie rozliczenia bez fiat, emisję po zgodnej wpłacie, dwa współbieżne
żądania rozliczenia, odczyt egzekwowalnego stanu tokenu oraz wykup 1:1 z
równoległym ponowieniem rozliczenia.

Po uruchomieniu Compose wykonaj:

```powershell
python .\scripts\run-p0-api-tests.py
```

Skrypt jest mutujący: emituje niewielką liczbę tokenów na deterministyczne konto
Hardhat, wykonuje odpowiadające wpłaty w mockBanku i spala część tokenów przy
wykupie. Każdy przebieg używa nowych identyfikatorów, więc pozostawione rekordy
są audytowalne i nie kolidują z wcześniejszym wykonaniem. Nie należy uruchamiać
go wobec sieci publicznej ani środowiska produkcyjnego.

Raport JSON trafia do `test-results/api-p0-issuer-*.json`. EM-04 na tym poziomie
potwierdza publikację stanu używanego przez bramkę operacji. Pełna macierz
`active`, `warning`, `mint_blocked`, `data_unavailable` i współbieżność przy
granicy rezerwy pozostaje objęta izolowanymi testami integracyjnymi backendu,
ponieważ wymuszanie tych stanów na współdzielonym demo zakłócałoby inne testy.
