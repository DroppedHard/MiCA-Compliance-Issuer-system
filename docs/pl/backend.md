# Backend emitenta i mockBank

Backend Rust obserwuje kontrakt i mockBank, zapisuje dane w SQLite oraz udostępnia HTTP i SSE. Odpowiada za podaż, estymaty ESG, pokrycie rezerw, zlecenia emisji i wykupu, wygaszanie, blokady adresów i raporty CASP.

## Struktura kodu

`src/api` jest rozdzielone na budowanie routera, trasy według obszaru, modele żądań, walidatory, handlery oraz odpowiedzi błędów. Handlery delegują przypadki użycia do `src/services`; nie realizują bezpośrednio SQL ani wywołań blockchaina.

Modele i reguły biznesowe pozostają w warstwie domenowej, a adaptery portów są pogrupowane w `src/infrastructure/sqlite`, `src/infrastructure/blockchain`, `src/infrastructure/bank` i `src/infrastructure/casp`. Ten podział utrzymuje osobno logikę emitenta, trwałość danych oraz połączenia z systemami zewnętrznymi.

## Planowane OpenAPI

Kontrakty HTTP są obecnie utrzymywane ręcznie pomiędzy backendem, frontendem i CASP. Planowanym rozszerzeniem jest generowanie specyfikacji OpenAPI z tras, modeli żądań, odpowiedzi i błędów. Ma ona ułatwić integrację i weryfikację kontraktów, ale nie zastępuje obecnie ich ręcznego utrzymywania i nie jest częścią bieżącej implementacji demonstracyjnej.

mockBank jest osobnym procesem tego samego projektu. Zaczyna od 0 USD, a emitent inicjalizuje demonstracyjne pokrycie 110% aktualnej podaży. Dalsza emisja zwiększa rezerwę i zobowiązanie 1:1.

## Start bez Dockera

Skopiuj `.env.example` do `.env`, ustaw `TOKEN_ADDRESS` i uruchom:

```powershell
cargo run --bin mock-bank
cargo run --bin crypto-asset-backend
```

Najważniejsze endpointy to `/api/v1/token`, `/esg`, `/reserves`, `/asset-state`, zlecenia `issuance-orders` i `redemption-orders` oraz administracyjne operacje rezerw, wygaszania i czarnej listy.

Emisja następuje dopiero po pasującej wpłacie fiat. Identyfikator operacji zapewnia idempotencję, a nieudana emisja po wpłacie powoduje kompensacyjny zwrot. Wykup spala token i wypłaca nominalnie 1 USD za 1 rUSD.

```powershell
cargo fmt --all --check
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```
