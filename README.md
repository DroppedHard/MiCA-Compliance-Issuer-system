# Emitent rUSD

[English version](README.en.md)

Repozytorium zawiera demonstracyjny system emitenta tokenu pieniądza elektronicznego rUSD powiązanego z dolarem amerykańskim. Pokazuje emisję i wykup, monitorowanie podaży i rezerw, stany tokenu, ograniczenia adresów oraz publikację informacji. Projekt ma charakter badawczy i nie stanowi produkcyjnej implementacji ani deklaracji zgodności z MiCA.

## Elementy systemu

- `asset/` — kontrakt `ResearchUsdEMT`, wdrożenie Hardhat i skrypty symulacyjne;
- `backend/` — API emitenta w Rust, obserwator blockchaina, SQLite i osobny proces mockBank;
- `frontend/` — panel administracyjny i publiczny dokument informacyjny;
- `docs/` — dokładniejsze omówienie architektury i kontraktu;
- `compose.yaml` — samodzielne lokalne wdrożenie emitenta.

CASP jest odrębnym systemem. Komunikuje się z emitentem przez HTTP i kontrakt rUSD; nie ma dostępu do bazy danych emitenta.

## Szybkie uruchomienie

Wymagany jest uruchomiony Docker Desktop. W katalogu repozytorium wykonaj:

```powershell
docker compose up --build --detach --wait
```

Dostępne adresy:

- panel emitenta: `http://127.0.0.1:5173`;
- dokument informacyjny: `http://127.0.0.1:5173/white-paper`;
- API emitenta: `http://127.0.0.1:3000`;
- mockBank: `http://127.0.0.1:3100`;
- Ethereum JSON-RPC: `http://127.0.0.1:8545`.

Zatrzymanie i wyzerowanie danych:

```powershell
docker compose down
docker compose down --volumes
```

Compose zawiera wyłącznie publiczne, deterministyczne konta Hardhat. Nie wolno używać tych kluczy w sieci publicznej ani umieszczać na nich rzeczywistych środków.

## Praca bez Dockera i weryfikacja

Szczegółowe polecenia zawierają README komponentów. Podstawowa weryfikacja obejmuje `cargo test`, testy Hardhat oraz test i produkcyjny build frontendu. Repozytorium celowo nie zawiera GitHub Actions — jest przygotowane jako samodzielny załącznik do pracy.

## Testy API uruchomionego emitenta

Scenariusze P0 są wykonywane względem rzeczywiście uruchomionych kontenerów
emitenta i mockBanku. Zalecane jest odizolowane środowisko wirtualne:

```powershell
python -m venv .venv
.\.venv\Scripts\python.exe -m pip install -r .\scripts\lib.txt
```

Manifest `scripts/lib.txt` nie zawiera obecnie pakietów zewnętrznych, ponieważ
runner korzysta wyłącznie z biblioteki standardowej. Pozostaje częścią
repozytorium, aby sposób przygotowania środowiska był stabilny po ewentualnym
dodaniu zależności.

Po uruchomieniu Compose wykonaj:

```powershell
.\.venv\Scripts\python.exe .\scripts\run-p0-api-tests.py
```

Test obejmuje przykłady EM-01–EM-05 i jest mutujący: pozostawia niewielkie
operacje bankowe, emisje, spalenie oraz rekordy audytowe. Każdy przebieg używa
nowych identyfikatorów. Niezerowy kod wyjścia oznacza błąd, a raport trafia do
`test-results/api-p0-issuer-*.json`. Szczegóły opisano w dokumencie
[scenariuszy API P0 emitenta](docs/pl/p0-api-tests.md).

## Dalsza dokumentacja

- [indeks dokumentacji emitenta](docs/pl/README.md);
- [model i stany kontraktu](docs/pl/token-contract.md);
- [kontrakt i skrypty blockchainowe](docs/pl/asset.md);
- [backend, mockBank i endpointy](docs/pl/backend.md);
- [scenariusze API P0 emitenta](docs/pl/p0-api-tests.md);
- [frontend emitenta](docs/pl/frontend.md).
