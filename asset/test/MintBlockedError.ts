import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { mintBlockedError, type MintStateSnapshot } from "../scripts/lib/mint-errors.js";

const ZERO = `0x${"00".repeat(32)}` as const;
const EVIDENCE = `0x${"ab".repeat(32)}` as const;

function active(overrides: Partial<MintStateSnapshot> = {}): MintStateSnapshot {
  return {
    windDown: false,
    paused: false,
    activityBlocked: false,
    activityEvidence: ZERO,
    reserveState: 0,
    reserveEvidence: ZERO,
    ...overrides,
  };
}

describe("MintBlockedError", () => {
  it("adds the activity-threshold reason and evidence hash", () => {
    const error = mintBlockedError(
      active({ activityBlocked: true, activityEvidence: EVIDENCE }),
    );
    assert.equal(error?.code, "ACTIVITY_THRESHOLD_BLOCK");
    assert.match(error?.message ?? "", /progów aktywności/);
    assert.match(error?.message ?? "", new RegExp(EVIDENCE));
  });

  it("distinguishes reserve blocking from wind-down and pause", () => {
    assert.equal(
      mintBlockedError(active({ reserveState: 2, reserveEvidence: EVIDENCE }))?.code,
      "RESERVE_COVERAGE_BLOCK",
    );
    assert.equal(mintBlockedError(active({ windDown: true }))?.code, "TOKEN_WIND_DOWN");
    assert.equal(mintBlockedError(active({ paused: true }))?.code, "TOKEN_PAUSED");
  });

  it("does not create an error for active or warning reserve states", () => {
    assert.equal(mintBlockedError(active()), undefined);
    assert.equal(mintBlockedError(active({ reserveState: 1 })), undefined);
  });
});
