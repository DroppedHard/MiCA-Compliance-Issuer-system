import { formatUnits, isAddress, parseUnits, zeroHash } from "viem";
import { network } from "hardhat";

const DEFAULT_TOKEN_ADDRESS = "0x5FbDB2315678afecb367f032d93F642f64180aa3";
const tokenAddress = addressFromEnvironment("TOKEN_ADDRESS", DEFAULT_TOKEN_ADDRESS);
const issuerUrl = (process.env.ISSUER_URL ?? "http://127.0.0.1:3000").replace(/\/$/, "");
const mockBankUrl = (process.env.MOCK_BANK_URL ?? "http://127.0.0.1:3100").replace(/\/$/, "");
const year = positiveInteger("THRESHOLD_YEAR", 2026);
const quarter = quarterNumber(process.env.THRESHOLD_QUARTER ?? "2");
const testBalance = parseUnits("1", 6);

const { viem } = await network.create("localhost");
const publicClient = await viem.getPublicClient();
const [admin, holder, recipient] = await viem.getWalletClients();
const token = await viem.getContractAt("ResearchUsdEMT", tokenAddress);

console.log("=== Scenariusz automatycznej blokady emisji ===");
console.log(`Kontrakt: ${tokenAddress}`);
console.log(`Kwartał syntetyczny: Q${quarter} ${year}`);

if (await token.read.issuanceBlocked()) {
  throw new Error("Emisja jest już zablokowana. Uruchom scripts/reset-demo.ps1 i spróbuj ponownie na świeżym wdrożeniu.");
}

const fundingHash = await token.write.mint([holder.account.address, testBalance], {
  account: admin.account,
});
await publicClient.waitForTransactionReceipt({ hash: fundingHash });
console.log(`Kontrola przed blokadą: mint ${formatUnits(testBalance, 6)} rUSD zakończony.`);

const response = await fetch(
  `${issuerUrl}/api/v1/admin/demo/casp-threshold-breach?year=${year}&quarter=${quarter}`,
  { method: "POST" },
);
const responseBody = await response.text();
if (!response.ok) {
  throw new Error(`Backend emitenta zwrócił HTTP ${response.status}: ${responseBody}`);
}
const assessment = JSON.parse(responseBody) as {
  averageDailyOperationCount: number;
  averageDailyValueEur: number;
  completeSourceRange: boolean;
  thresholdBreached: boolean;
  thresholdEnforceable: boolean;
};
if (!assessment.completeSourceRange || !assessment.thresholdBreached || !assessment.thresholdEnforceable) {
  throw new Error(`Ocena nie uruchomiła blokady: ${responseBody}`);
}
console.log(`Średnia dzienna liczba transakcji: ${assessment.averageDailyOperationCount}`);
console.log(`Średnia dzienna wartość: ${assessment.averageDailyValueEur.toFixed(2)} EUR`);

if (!(await token.read.issuanceBlocked())) {
  throw new Error("Backend zapisał ocenę, ale kontrakt nie ustawił issuanceBlocked=true.");
}
const evidence = await token.read.issuanceBlockEvidence();
if (evidence === zeroHash) {
  throw new Error("Kontrakt nie zapisał skrótu dowodu kwartalnego.");
}
console.log(`Blokada on-chain aktywna. Skrót dowodu: ${evidence}`);

const blockedOperationId = `threshold-block-check-${year}-q${quarter}`;
await expectSuccess(`${issuerUrl}/api/v1/issuance-orders`, {
  method: "POST",
  headers: { "content-type": "application/json" },
  body: JSON.stringify({
    operationId: blockedOperationId,
    recipientAddress: recipient.account.address,
    amountUsdMinor: "1",
  }),
});
await expectSuccess(`${mockBankUrl}/api/v1/reserve-accounts/reserve-rusd/deposits`, {
  method: "POST",
  headers: { "content-type": "application/json" },
  body: JSON.stringify({
    amountMinor: "1",
    reference: blockedOperationId,
    idempotencyKey: `issuance-${blockedOperationId}`,
  }),
});
const settlement = await fetch(`${issuerUrl}/api/v1/issuance-orders/${blockedOperationId}/settle`, {
  method: "POST",
});
if (settlement.status !== 409) {
  throw new Error(`Backend nie odrzucił emisji po blokadzie; otrzymano HTTP ${settlement.status}: ${await settlement.text()}`);
}
console.log("Kontrola backendu: emisja z potwierdzoną wpłatą została odrzucona przez centralną bramkę.");
await expectSuccess(`${issuerUrl}/api/v1/admin/reserves/adjustments`, {
  method: "POST",
  headers: { "content-type": "application/json" },
  body: JSON.stringify({
    operationId: `${blockedOperationId}-reserve-cleanup`,
    direction: "withdrawal",
    amountUsd: "0.01",
    reason: "Usunięcie technicznej wpłaty po kontroli zablokowanej emisji",
  }),
});

let mintRejected = false;
try {
  await token.write.mint([recipient.account.address, 1n], { account: admin.account });
} catch (error) {
  mintRejected = String(error).includes("IssuanceBlocked");
}
if (!mintRejected) throw new Error("Bezpośredni mint nie został odrzucony błędem IssuanceBlocked.");
console.log("Kontrola negatywna: bezpośredni mint został odrzucony przez kontrakt.");

const transferred = parseUnits("0.25", 6);
const transferHash = await token.write.transfer([recipient.account.address, transferred], {
  account: holder.account,
});
await publicClient.waitForTransactionReceipt({ hash: transferHash });
const holderBurnHash = await token.write.burn([holder.account.address, testBalance - transferred], {
  account: admin.account,
});
await publicClient.waitForTransactionReceipt({ hash: holderBurnHash });
const recipientBurnHash = await token.write.burn([recipient.account.address, transferred], {
  account: admin.account,
});
await publicClient.waitForTransactionReceipt({ hash: recipientBurnHash });
console.log("Kontrola zakresu: transfer oraz burn nadal działają po blokadzie emisji.");
console.log("SCENARIUSZ ZAKOŃCZONY POWODZENIEM");

function addressFromEnvironment(name: string, fallback: string): `0x${string}` {
  const value = process.env[name] ?? fallback;
  if (!isAddress(value)) throw new Error(`${name} nie zawiera poprawnego adresu kontraktu.`);
  return value;
}

function positiveInteger(name: string, fallback: number): number {
  const value = process.env[name];
  if (value === undefined) return fallback;
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed <= 0) throw new Error(`${name} musi być dodatnią liczbą całkowitą.`);
  return parsed;
}

function quarterNumber(value: string): number {
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < 1 || parsed > 4) throw new Error("THRESHOLD_QUARTER musi mieścić się w zakresie 1-4.");
  return parsed;
}

async function expectSuccess(url: string, init: RequestInit): Promise<void> {
  const response = await fetch(url, init);
  if (!response.ok) throw new Error(`HTTP ${response.status} dla ${url}: ${await response.text()}`);
}
