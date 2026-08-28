import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { network } from "hardhat";
import { keccak256, parseUnits, toBytes } from "viem";

describe("CaspDepositRouter", () => {
  it("moves rUSD directly to hot custody and emits the logical client reference", async () => {
    const { viem } = await network.connect();
    const [admin, sender, hot] = await viem.getWalletClients();
    const token = await viem.deployContract("ResearchUsdEMT", [admin.account.address]);
    const router = await viem.deployContract("CaspDepositRouter", [token.address, hot.account.address]);
    const amount = parseUnits("25", 6);
    const reference = keccak256(toBytes("rusd:casp:alice"));
    await token.write.mint([sender.account.address, amount]);
    await token.write.approve([router.address, amount], { account: sender.account });

    await viem.assertions.emitWithArgs(router.write.depositFor([reference, amount], { account: sender.account }), router, "DepositReceived", [sender.account.address, reference, amount]);
    assert.equal(await token.read.balanceOf([hot.account.address]), amount);
    assert.equal(await token.read.balanceOf([router.address]), 0n);
  });
});
