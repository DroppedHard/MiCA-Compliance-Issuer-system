# Kontrakt i skrypty blockchainowe

## Instalacja i testy

```powershell
npm.cmd install
npm.cmd run typecheck
npm.cmd test
npm.cmd run build
```

## Lokalna sieć

Uruchom `npx.cmd hardhat node`, a w drugim terminalu:

```powershell
npx.cmd hardhat ignition deploy ignition/modules/ResearchUsdEMT.ts --network localhost
```

Przekaż wydrukowany adres backendowi jako `TOKEN_ADDRESS`.

## Scenariusze

Po ustawieniu adresu kontraktu dostępne są:

```powershell
npm.cmd run traffic
npm.cmd run supply-cycle
npm.cmd run compliance-threshold
npm.cmd run external-deposit -- alice 100
```

Skrypty bezpośredniego mintowania celowo omijają bramkę emitenta i służą wyłącznie do testów.

