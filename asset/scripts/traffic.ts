import { formatUnits, isAddress, parseUnits } from "viem";
import { network } from "hardhat";
import { executeMintWithContext } from "./lib/mint-errors.js";

const DECIMALS = 6;
const STARTING_BALANCE = parseUnits("1000", DECIMALS);
const DEFAULT_INTERVAL_MS = 3_000;

const tokenAddress = requiredAddress("TOKEN_ADDRESS");
const intervalMs = positiveInteger("TRAFFIC_INTERVAL_MS", DEFAULT_INTERVAL_MS);
const maxTransfers = optionalPositiveInteger("TRAFFIC_MAX_TRANSFERS");

const { viem } = await network.create("localhost");
const publicClient = await viem.getPublicClient();
const [admin, ...availableUsers] = await viem.getWalletClients();
const users = availableUsers.slice(0, 4);

if (users.length < 4) {
  throw new Error("The traffic simulator requires at least five unlocked local accounts.");
}

const token = await viem.getContractAt("ResearchUsdEMT", tokenAddress);

console.log(`Connected to ResearchUsdEMT at ${tokenAddress}`);
console.log(`Preparing ${users.length} simulated users...`);

for (const user of users) {
  const balance = await token.read.balanceOf([user.account.address]);
  if (balance >= STARTING_BALANCE) continue;

  const missingAmount = STARTING_BALANCE - balance;
  const hash = await executeMintWithContext(token, () =>
    token.write.mint([user.account.address, missingAmount], {
      account: admin.account,
    }),
  );
  await publicClient.waitForTransactionReceipt({ hash });
  console.log(
    `Funded ${shortAddress(user.account.address)} with ${formatUnits(missingAmount, DECIMALS)} rUSD`,
  );
}

console.log(
  `Traffic started: one transfer every ${intervalMs} ms${maxTransfers === undefined ? " until Ctrl+C" : `, ${maxTransfers} transfers`}.`,
);

let shouldStop = false;
let completedTransfers = 0;

process.on("SIGINT", () => {
  shouldStop = true;
  console.log("\nStopping after the current transfer...");
});

while (!shouldStop && (maxTransfers === undefined || completedTransfers < maxTransfers)) {
  const sender = users[completedTransfers % users.length];
  const recipient = users[(completedTransfers + 1) % users.length];
  const amount = parseUnits(String((completedTransfers % 5) + 1), DECIMALS);

  const hash = await token.write.transfer([recipient.account.address, amount], {
    account: sender.account,
  });
  const receipt = await publicClient.waitForTransactionReceipt({ hash });
  completedTransfers += 1;

  console.log(
    `#${completedTransfers} block=${receipt.blockNumber} ${shortAddress(sender.account.address)} -> ${shortAddress(recipient.account.address)} amount=${formatUnits(amount, DECIMALS)} rUSD tx=${hash}`,
  );

  if (!shouldStop && (maxTransfers === undefined || completedTransfers < maxTransfers)) {
    await delay(intervalMs);
  }
}

console.log(`Traffic stopped after ${completedTransfers} transfers.`);

function requiredAddress(name: string): `0x${string}` {
  const value = process.env[name];
  if (value === undefined || !isAddress(value)) {
    throw new Error(`${name} must contain a valid deployed contract address.`);
  }
  return value;
}

function positiveInteger(name: string, fallback: number): number {
  const value = process.env[name];
  if (value === undefined) return fallback;
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed <= 0) {
    throw new Error(`${name} must be a positive integer.`);
  }
  return parsed;
}

function optionalPositiveInteger(name: string): number | undefined {
  const value = process.env[name];
  if (value === undefined) return undefined;
  return positiveInteger(name, 1);
}

function shortAddress(address: string): string {
  return `${address.slice(0, 8)}...${address.slice(-6)}`;
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
