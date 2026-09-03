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

## Dalsza dokumentacja

- [indeks dokumentacji emitenta](docs/pl/README.md);
- [model i stany kontraktu](docs/pl/token-contract.md);
- [kontrakt i skrypty blockchainowe](docs/pl/asset.md);
- [backend, mockBank i endpointy](docs/pl/backend.md);
- [frontend emitenta](docs/pl/frontend.md).
