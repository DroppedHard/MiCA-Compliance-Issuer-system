import { describe, expect, it } from "vitest"
import { resolveAppView } from "./app"

describe("application view routing", () => {
  it("opens the customer portal only for the client path", () => {
    expect(resolveAppView("/client")).toBe("client")
    expect(resolveAppView("/client/operations/123")).toBe("client")
    expect(resolveAppView("/")).toBe("admin")
    expect(resolveAppView("/unknown")).toBe("admin")
  })
})
