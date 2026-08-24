import { Effect, Schema } from "effect"
import { describe, expect, it } from "vitest"
import { ReserveCoverageSchema } from "./reserve"
import { appendReserveObservation, decodeReserveEventData } from "@/application/realtime/use-reserve-stream"

const coverage={observedAtUnixMs:2,bankAsOfUnixMs:1,reserveAccountId:"reserve-rusd",currency:"USD",reserveBalanceMinor:"450000",reserveBalanceUsd:"4500.00",tokenSupplyRaw:"4300000000",liabilityUsd:"4300",surplusUsd:"200",ratioPercent:104.65,status:"covered" as const}
describe("reserve coverage contract",()=>{
  it("accepts backend data and SSE payloads",async()=>{ expect((await Effect.runPromise(Schema.decodeUnknown(ReserveCoverageSchema)(coverage))).status).toBe("covered"); await expect(decodeReserveEventData(JSON.stringify(coverage))).resolves.toMatchObject({surplusUsd:"200"}) })
  it("rejects an unknown coverage status",async()=>{ await expect(Effect.runPromise(Schema.decodeUnknown(ReserveCoverageSchema)({...coverage,status:"unknown"}))).rejects.toBeDefined() })
  it("keeps only the newest observations without mutating previous history",()=>{
    const first = { ...coverage, observedAtUnixMs: 1 }
    const second = { ...coverage, observedAtUnixMs: 2 }
    const third = { ...coverage, observedAtUnixMs: 3 }
    const previous = [first, second]

    const result = appendReserveObservation(previous, third, 2)

    expect(result.map(value=>value.observedAtUnixMs)).toEqual([2,3])
    expect(previous.map(value=>value.observedAtUnixMs)).toEqual([1,2])
  })
})
