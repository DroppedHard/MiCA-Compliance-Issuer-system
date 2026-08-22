import { describe, expect, it } from "vitest"
import { formatTokenAmount, shortenAddress } from "./token"

describe("token presentation transforms", () => {
  it("formats raw six-decimal token amounts", () => {
    expect(formatTokenAmount("1000000", 6)).toBe("1")
    expect(formatTokenAmount("1250000", 6)).toBe("1.25")
  })

  it("shortens an Ethereum address", () => {
    expect(shortenAddress("0x1234567890abcdef")).toBe("0x123456…abcdef")
  })
})
