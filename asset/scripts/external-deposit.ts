import { network } from "hardhat";
import { keccak256, parseUnits, toBytes } from "viem";
import { executeMintWithContext } from "./lib/mint-errors.js";

const TOKEN_ADDRESS = (process.env.TOKEN_ADDRESS ?? "0x5FbDB2315678afecb367f032d93F642f64180aa3") as `0x${string}`;
const ROUTER_ADDRESS = (process.env.CASP_DEPOSIT_ROUTER_ADDRESS ?? "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512") as `0x${string}`;
const client = process.env.CASP_CLIENT_ID ?? "alice";
const amountText = process.env.DEPOSIT_AMOUNT_RUSD ?? "100";
const knownClients = new Set(["alice", "bob", "carol"]);

if (!knownClients.has(client)) {
  throw new Error("Unknown demo client. Use: alice, bob or carol.");
}

const logicalAddress = `rusd:casp:${client}`;
const amount = parseUnits(amountText, 6);
if (amount <= 0n) {
  throw new Error("Deposit amount must be greater than zero.");
}

const { viem, networkHelpers } = await network.connect();
const wallets = await viem.getWalletClients();
const issuer = wallets[0];
const externalSender = wallets[4];
if (issuer === undefined || externalSender === undefined) {
  throw new Error("The local Hardhat node must expose at least five deterministic accounts.");
}
const publicClient = await viem.getPublicClient();
const token = await viem.getContractAt("ResearchUsdEMT", TOKEN_ADDRESS);
const router = await viem.getContractAt("CaspDepositRouter", ROUTER_ADDRESS);
const reference = keccak256(toBytes(logicalAddress));

// Demo funding only: direct mint deliberately bypasses the issuer workflow.
const mintHash = await executeMintWithContext(token, () =>
  token.write.mint([externalSender.account.address, amount], { account: issuer.account }),
);
await publicClient.waitForTransactionReceipt({ hash: mintHash });

const approvalHash = await token.write.approve([ROUTER_ADDRESS, amount], { account: externalSender.account });
await publicClient.waitForTransactionReceipt({ hash: approvalHash });

const transactionHash = await router.write.depositFor([reference, amount], { account: externalSender.account });
await publicClient.waitForTransactionReceipt({ hash: transactionHash });

const confirmationBlocks = Number(process.env.CASP_DEPOSIT_CONFIRMATIONS ?? "2");
if (!Number.isSafeInteger(confirmationBlocks) || confirmationBlocks < 0) {
  throw new Error("CASP_DEPOSIT_CONFIRMATIONS must be a non-negative integer.");
}
if (confirmationBlocks > 0) {
  await networkHelpers.mine(confirmationBlocks);
}

console.log("External CASP deposit submitted.");
console.log({
  client,
  logicalAddress,
  clientReferenceHash: reference,
  externalSender: externalSender.account.address,
  amountRUSD: amountText,
  amountRaw: amount.toString(),
  transactionHash,
  confirmationBlocksMined: confirmationBlocks,
});
console.log("CASP should credit the logical account after its next observer poll (up to about 5 seconds).");
