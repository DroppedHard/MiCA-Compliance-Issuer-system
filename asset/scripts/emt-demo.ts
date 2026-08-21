import { formatUnits } from "viem";
import { network } from "hardhat";

const { viem } = await network.create();
const [admin, holder, merchant] = await viem.getWalletClients();
const token = await viem.deployContract("ResearchEuroEMT", [admin.account.address]);
await token.write.mint([holder.account.address, 100n * 10n ** 6n]);
await token.write.transfer([merchant.account.address, 25n * 10n ** 6n], { account: holder.account });
await token.write.burn([merchant.account.address, 5n * 10n ** 6n]);
console.log("ResearchEuroEMT deployed at:", token.address);
console.log("Holder balance:", formatUnits(await token.read.balanceOf([holder.account.address]), 6), "rEUR");
console.log("Merchant balance:", formatUnits(await token.read.balanceOf([merchant.account.address]), 6), "rEUR");
console.log("Total supply:", formatUnits(await token.read.totalSupply(), 6), "rEUR");
