import { formatUnits, isAddress, parseUnits } from "viem";
import { network } from "hardhat";

const DECIMALS = 6;
const DEFAULT_MINT_AMOUNT = "500";
const DEFAULT_BURN_AMOUNT = "200";
const DEFAULT_DELAY_MS = 12_000;

const tokenAddress = requiredAddress("TOKEN_ADDRESS");
const mintAmount = tokenAmount("SUPPLY_MINT_AMOUNT", DEFAULT_MINT_AMOUNT);
const burnAmount = tokenAmount("SUPPLY_BURN_AMOUNT", DEFAULT_BURN_AMOUNT);
const delayMs = nonNegativeInteger("SUPPLY_DELAY_MS", DEFAULT_DELAY_MS);

if (burnAmount > mintAmount) {
  throw new Error("SUPPLY_BURN_AMOUNT cannot exceed SUPPLY_MINT_AMOUNT.");
}

const { viem } = await network.create("localhost");
const publicClient = await viem.getPublicClient();
const [admin, holder] = await viem.getWalletClients();
const token = await viem.getContractAt("ResearchEuroEMT", tokenAddress);

const initialSupply = await token.read.totalSupply();
console.log(`Connected to ResearchEuroEMT at ${tokenAddress}`);
console.log(`Initial supply: ${formatUnits(initialSupply, DECIMALS)} rEUR`);

const mintHash = await token.write.mint([holder.account.address, mintAmount], {
  account: admin.account,
});
const mintReceipt = await publicClient.waitForTransactionReceipt({ hash: mintHash });
const supplyAfterMint = await token.read.totalSupply();
console.log(
  `Minted ${formatUnits(mintAmount, DECIMALS)} rEUR in block ${mintReceipt.blockNumber}. Supply: ${formatUnits(supplyAfterMint, DECIMALS)} rEUR`,
);

if (delayMs > 0) {
  console.log(`Waiting ${delayMs} ms so the backend can observe the increased supply...`);
  await delay(delayMs);
}

const burnHash = await token.write.burn([holder.account.address, burnAmount], {
  account: admin.account,
});
const burnReceipt = await publicClient.waitForTransactionReceipt({ hash: burnHash });
const finalSupply = await token.read.totalSupply();
console.log(
  `Burned ${formatUnits(burnAmount, DECIMALS)} rEUR in block ${burnReceipt.blockNumber}. Final supply: ${formatUnits(finalSupply, DECIMALS)} rEUR`,
);
console.log(`Net supply change: +${formatUnits(mintAmount - burnAmount, DECIMALS)} rEUR`);

function requiredAddress(name: string): `0x${string}` {
  const value = process.env[name];
  if (value === undefined || !isAddress(value)) {
    throw new Error(`${name} must contain a valid deployed contract address.`);
  }
  return value;
}

function tokenAmount(name: string, fallback: string): bigint {
  const value = process.env[name] ?? fallback;
  try {
    const parsed = parseUnits(value, DECIMALS);
    if (parsed <= 0n) throw new Error();
    return parsed;
  } catch {
    throw new Error(`${name} must be a positive token amount, got: ${value}`);
  }
}

function nonNegativeInteger(name: string, fallback: number): number {
  const value = process.env[name];
  if (value === undefined) return fallback;
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < 0) {
    throw new Error(`${name} must be a non-negative integer.`);
  }
  return parsed;
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
