import { Effect, Schema } from "effect"
import { describe, expect, it } from "vitest"
import { EsgObservationSchema } from "./esg"
import { decodeEsgEventData, mergeEsgHistory } from "@/application/realtime/use-esg-stream"
import { toEsgChartPoints } from "@/features/dashboard/esg-energy-chart"

const validObservation = {
  observedAtUnixMs: 123,
  lastProcessedBlock: 42,
  chainId: 1,
  contractAddress: "0xabc",
  currentDay: {
    dateUtc: "2026-08-22", status: "provisional", dataOrigin: "observed", transactionCount: 2,
    energyLowerWh: 6.3, energyBestGuessWh: 39.35, energyUpperWh: 57.45, emissionsGCo2e: 11.85,
    renewableEnergyWh: 15.46455, nuclearEnergyWh: 6.6895, fossilEnergyWh: 17.1566,
  },
  methodology: {
    version: "ccaf-ethereum-pos-2026-demo-v1",
    annualTransactionsAssumption: 400000000,
    lowerEnergyWhPerTransaction: 3.15, bestGuessEnergyWhPerTransaction: 19.675, upperEnergyWhPerTransaction: 28.725,
    emissionsGCo2ePerTransaction: 5.925,
    renewablePercent: 39.3, nuclearPercent: 17, fossilPercent: 43.6,
    sourceName: "Cambridge", sourceUrl: "https://example.test/report.pdf", note: "Estimate",
  },
}

describe("ESG API contract", () => {
  it("accepts the complete backend observation", async () => {
    const decoded = await Effect.runPromise(Schema.decodeUnknown(EsgObservationSchema)(validObservation))
    expect(decoded.currentDay.transactionCount).toBe(2)
    expect(decoded.methodology.renewablePercent).toBe(39.3)
  })

  it("rejects an incomplete methodology", async () => {
    const invalid = { ...validObservation, methodology: { version: "v1" } }
    await expect(Effect.runPromise(Schema.decodeUnknown(EsgObservationSchema)(invalid))).rejects.toBeDefined()
  })

  it("decodes the JSON payload received through SSE", async () => {
    await expect(decodeEsgEventData(JSON.stringify(validObservation))).resolves.toMatchObject({
      lastProcessedBlock: 42,
      currentDay: { energyBestGuessWh: 39.35 },
    })
  })

  it("rejects malformed SSE data", async () => {
    await expect(decodeEsgEventData("not-json")).rejects.toBeDefined()
  })

  it("replaces the current day in history instead of duplicating it", async () => {
    const observation = await decodeEsgEventData(JSON.stringify(validObservation))
    const older = { ...observation.currentDay, dateUtc: "2026-08-21", transactionCount: 1 }
    const staleCurrent = { ...observation.currentDay, transactionCount: 1 }
    const merged = mergeEsgHistory({ days: [staleCurrent, older], methodology: observation.methodology }, observation)

    expect(merged?.days.map((day) => day.dateUtc)).toEqual(["2026-08-21", "2026-08-22"])
    expect(merged?.days.at(-1)?.transactionCount).toBe(2)
  })

  it("maps lower, best and upper values to the chart without losing their order", async () => {
    const observation = await decodeEsgEventData(JSON.stringify(validObservation))
    expect(toEsgChartPoints([observation.currentDay])[0]).toMatchObject({
      range: [6.3, 57.45], best: 39.35, origin: "observed",
    })
  })
})
