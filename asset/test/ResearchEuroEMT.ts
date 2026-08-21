import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { network } from "hardhat";

describe("ResearchEuroEMT integration", async function () {
  const { viem } = await network.create();
  const [admin, holder, recipient] = await viem.getWalletClients();

  it("runs an issuance, payment and redemption flow", async function () {
    const token = await viem.deployContract("ResearchEuroEMT", [admin.account.address]);
    const issued = 100n * 10n ** 6n;
    const payment = 35n * 10n ** 6n;
    const redemption = 15n * 10n ** 6n;
    await viem.assertions.emitWithArgs(token.write.mint([holder.account.address, issued]), token, "Transfer", ["0x0000000000000000000000000000000000000000", holder.account.address, issued]);
    await token.write.transfer([recipient.account.address, payment], { account: holder.account });
    await viem.assertions.emitWithArgs(token.write.burn([recipient.account.address, redemption]), token, "Transfer", [recipient.account.address, "0x0000000000000000000000000000000000000000", redemption]);
    assert.equal(await token.read.balanceOf([holder.account.address]), issued - payment);
    assert.equal(await token.read.balanceOf([recipient.account.address]), payment - redemption);
    assert.equal(await token.read.totalSupply(), issued - redemption);
  });

  it("blocks payments involving a frozen address", async function () {
    const token = await viem.deployContract("ResearchEuroEMT", [admin.account.address]);
    await token.write.mint([holder.account.address, 10n * 10n ** 6n]);
    await viem.assertions.emitWithArgs(token.write.freeze([holder.account.address]), token, "AddressFrozen", [holder.account.address]);
    await viem.assertions.revertWithCustomErrorWithArgs(token.write.transfer([recipient.account.address, 1n], { account: holder.account }), token, "AccountFrozen", [holder.account.address]);
  });
});
