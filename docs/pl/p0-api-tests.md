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

## Terminalny scenariusz wygaszania EM-07

Na końcu pracy ze świeżym, jednorazowym wdrożeniem można wykonać rozszerzony
przebieg:

```powershell
python .\scripts\run-p0-api-tests.py --terminal-wind-down
```

Flaga dodaje jako ostatni przypadek test EM-07. Scenariusz najpierw zapewnia
saldo testowego posiadacza i potwierdza zwykły transfer ERC-20, a następnie:

1. uruchamia `wind_down` przez API administratora emitenta;
2. ponawia tę samą komendę i potwierdza idempotencję stanu terminalnego;
3. sprawdza odrzucenie nowej emisji, kod `issuance_blocked` i zwrot fiat do CASP;
4. wykonuje bezpośrednią próbę transferu przez JSON-RPC i oczekuje odrzucenia
   przez kontrakt;
5. rozlicza wykup, potwierdzając dostępność spalenia tokenów i wypłatę 1:1.

Jest to test destrukcyjny dla cyklu życia tokenu: `wind_down` jest
nieodwracalny. Należy uruchamiać go jako ostatni test na lokalnym wdrożeniu,
a przed kolejną serią odtworzyć środowisko demonstracyjne wraz z wolumenami.
Domyślny przebieg bez flagi pozostaje bezpieczny dla dalszych testów.

Raport JSON trafia do `test-results/api-p0-issuer-*.json`. Przy użyciu flagi
zawiera pozycję EM-07 z poziomem `live-api-terminal` oraz informację, czy część
terminalna została wykonana. EM-04 na tym poziomie
potwierdza publikację stanu używanego przez bramkę operacji. Pełna macierz
`active`, `warning`, `mint_blocked`, `data_unavailable` i współbieżność przy
granicy rezerwy pozostaje objęta izolowanymi testami integracyjnymi backendu,
ponieważ wymuszanie tych stanów na współdzielonym demo zakłócałoby inne testy.
