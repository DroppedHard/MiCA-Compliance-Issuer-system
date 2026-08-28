import { network } from "hardhat";
import { keccak256, parseUnits, toBytes } from "viem";

const TOKEN_ADDRESS = (process.env.TOKEN_ADDRESS ?? "0x5FbDB2315678afecb367f032d93F642f64180aa3") as `0x${string}`;
const ROUTER_ADDRESS = (process.env.CASP_DEPOSIT_ROUTER_ADDRESS ?? "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512") as `0x${string}`;
const logicalAddress = process.env.CASP_CLIENT_REFERENCE ?? "rusd:casp:alice";
const amount = parseUnits(process.env.DEPOSIT_AMOUNT_RUSD ?? "100", 6);

const { viem } = await network.connect();
const [issuer, externalSender] = await viem.getWalletClients();
const token = await viem.getContractAt("ResearchUsdEMT", TOKEN_ADDRESS);
const router = await viem.getContractAt("CaspDepositRouter", ROUTER_ADDRESS);
const reference = keccak256(toBytes(logicalAddress));

await token.write.mint([externalSender.account.address, amount], { account: issuer.account });
await token.write.approve([ROUTER_ADDRESS, amount], { account: externalSender.account });
const transactionHash = await router.write.depositFor([reference, amount], { account: externalSender.account });
console.log({ logicalAddress, amountRaw: amount.toString(), transactionHash });
