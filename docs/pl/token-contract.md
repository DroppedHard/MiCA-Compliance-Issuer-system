# Kontrakt i stany rUSD

`ResearchUsdEMT` jest demonstracyjnym ERC-20 z sześcioma miejscami dziesiętnymi, sterowaną emisją i spalaniem oraz rozdzielonymi rolami. Inspiracją jest publiczna architektura Circle EURC, lecz projekt nie odtwarza kompletnego systemu produkcyjnego.

## Operacje i stany

- `mintForOperation` emituje token raz dla danego identyfikatora;
- `burn` zmniejsza podaż podczas wykupu;
- `freeze` blokuje operacje dotyczące wskazanego adresu;
- `pause` zatrzymuje wszystkie zmiany sald;
- `blockIssuance` blokuje tylko emisję na podstawie dowodu aktywności;
- `setReserveState` odwracalnie synchronizuje ocenę rezerw;
- `enterWindDown` nieodwracalnie blokuje emisję i zwykłe transfery, pozostawiając spalanie potrzebne do wykupu.

Efektywny stan to `Active`, `Warning`, `IssuanceBlocked` albo terminalny `WindDown`. Blokada aktywności i blokada rezerw są niezależne. Poprawa jednej osi nie usuwa ograniczenia pochodzącego z drugiej.

Role obejmują administratora, mintera, burnera, operatora pauzy, operatora blokad adresów, operatora ograniczeń emisji i operatora wygaszania. W demo konto Hardhat numer 0 otrzymuje wszystkie role; produkcyjnie wymagałyby rozdzielenia.

Kontrakt nie przechowuje fiat i nie potwierdza przelewów bankowych. Proces koordynuje backend emitenta.

