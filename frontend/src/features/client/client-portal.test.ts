import { describe, expect, it } from "vitest"
import { parsePositiveAmount } from "./client-portal"

describe("client operation amount", () => {
  it("accepts Polish and dot decimal formats", () => {
    expect(parsePositiveAmount("100,50")).toBe(100.5)
    expect(parsePositiveAmount("100.50")).toBe(100.5)
  })

  it("rejects invalid, zero and negative amounts", () => {
    expect(parsePositiveAmount("abc")).toBe(0)
    expect(parsePositiveAmount("0")).toBe(0)
    expect(parsePositiveAmount("-1")).toBe(0)
  })
})
