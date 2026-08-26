import { describe, expect, it } from "vitest"
import { currentPublication, publications } from "./content"

describe("white paper publication history",()=>{
  it("has exactly one current, timestamped immutable publication",()=>{
    expect(publications.filter(item=>item.status==="current")).toHaveLength(1)
    expect(currentPublication.version).toMatch(/^\d+\.\d+\.\d+-demo$/)
    expect(Number.isNaN(Date.parse(currentPublication.publishedAt))).toBe(false)
  })
})
