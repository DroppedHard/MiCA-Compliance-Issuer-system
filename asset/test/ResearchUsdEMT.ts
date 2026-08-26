import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { network } from "hardhat";

describe("ResearchUsdEMT integration", async function () {
  const { viem } = await network.create();
  const [admin, holder, recipient] = await viem.getWalletClients();

  it("runs an issuance, payment and redemption flow", async function () {
    const token = await viem.deployContract("ResearchUsdEMT", [admin.account.address]);
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
    const token = await viem.deployContract("ResearchUsdEMT", [admin.account.address]);
    await token.write.mint([holder.account.address, 10n * 10n ** 6n]);
    await viem.assertions.emitWithArgs(token.write.freeze([holder.account.address]), token, "AddressFrozen", [holder.account.address]);
    await viem.assertions.revertWithCustomErrorWithArgs(token.write.transfer([recipient.account.address, 1n], { account: holder.account }), token, "AccountFrozen", [holder.account.address]);
  });

  it("executes a correlated mint operation exactly once", async function () {
    const token = await viem.deployContract("ResearchUsdEMT", [admin.account.address]);
    const operationId = `0x${"11".repeat(32)}` as `0x${string}`;
    const amount = 25n * 10n ** 6n;

    await viem.assertions.emitWithArgs(
      token.write.mintForOperation([operationId, holder.account.address, amount]),
      token,
      "MintOperationExecuted",
      [operationId, holder.account.address, amount],
    );
    assert.equal(await token.read.isMintOperationProcessed([operationId]), true);
    await viem.assertions.revertWithCustomErrorWithArgs(
      token.write.mintForOperation([operationId, holder.account.address, amount]),
      token,
      "MintOperationAlreadyProcessed",
      [operationId],
    );
    assert.equal(await token.read.balanceOf([holder.account.address]), amount);
  });

  it("executes a correlated redemption burn exactly once", async function () {
    const token = await viem.deployContract("ResearchUsdEMT", [admin.account.address]);
    const operationId = `0x${"22".repeat(32)}` as `0x${string}`;
    const amount = 10n * 10n ** 6n;
    await token.write.mint([holder.account.address, amount]);
    await viem.assertions.emitWithArgs(
      token.write.burnForOperation([operationId, holder.account.address, amount]),
      token,
      "BurnOperationExecuted",
      [operationId, holder.account.address, amount],
    );
    assert.equal(await token.read.isBurnOperationProcessed([operationId]), true);
    await viem.assertions.revertWithCustomErrorWithArgs(
      token.write.burnForOperation([operationId, holder.account.address, amount]),
      token,
      "BurnOperationAlreadyProcessed",
      [operationId],
    );
  });

  it("makes wind-down terminal for mint and transfers while preserving burns", async function () {
    const token = await viem.deployContract("ResearchUsdEMT", [admin.account.address]);
    const amount = 10n * 10n ** 6n;
    await token.write.mint([holder.account.address, amount]);

    await viem.assertions.emitWithArgs(
      token.write.enterWindDown(),
      token,
      "WindDownEntered",
      [admin.account.address],
    );
    assert.equal(await token.read.windDown(), true);
    await viem.assertions.revertWithCustomError(
      token.write.mint([recipient.account.address, 1n]),
      token,
      "WindDownBlocksOperation",
    );
    await viem.assertions.revertWithCustomError(
      token.write.transfer([recipient.account.address, 1n], { account: holder.account }),
      token,
      "WindDownBlocksOperation",
    );
    await token.write.burn([holder.account.address, amount]);
    assert.equal(await token.read.balanceOf([holder.account.address]), 0n);

    // Repeated commands are idempotent and there is intentionally no exit function.
    await token.write.enterWindDown();
    assert.equal(await token.read.windDown(), true);
  });

  it("keeps the existing global pause stricter than wind-down", async function () {
    const token = await viem.deployContract("ResearchUsdEMT", [admin.account.address]);
    await token.write.mint([holder.account.address, 1n]);
    await token.write.pause();
    await viem.assertions.revertWithCustomError(
      token.write.burn([holder.account.address, 1n]),
      token,
      "EnforcedPause",
    );
  });
});
