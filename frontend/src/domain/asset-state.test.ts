import { describe, expect, it } from "vitest"
import { Schema } from "effect"
import { AssetStateSchema, assetStateLabel } from "./asset-state"

describe("asset state API contract", () => {
  it("decodes persisted issuer decisions", () => {
    const value = Schema.decodeUnknownSync(AssetStateSchema)({ state: "warning", reason: "margin", reserveCoveragePercent: 103, evidenceAtUnixMs: 1, policyVersion: "v1", updatedAtUnixMs: 2 })
    expect(value.state).toBe("warning")
    expect(assetStateLabel(value.state)).toBe("Ostrzeżenie")
  })
})
