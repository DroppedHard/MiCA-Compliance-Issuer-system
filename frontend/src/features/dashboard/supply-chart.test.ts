import { describe, expect, it } from "vitest"
import type { ReserveCoverage } from "@/domain/reserve"
import type { TokenObservation } from "@/domain/token"
import { buildSupplyReservePoints, selectChartWindow } from "./supply-chart"

const token = (observedAtUnixMs: number, totalSupplyRaw: string): TokenObservation => ({
  observedAtUnixMs,
  snapshot: {
    chainId: 31337,
    blockNumber: 1,
    contractAddress: "0x1",
    name: "Research USD EMT",
    symbol: "rUSD",
    decimals: 6,
    totalSupplyRaw,
  },
})

const reserve = (observedAtUnixMs: number, reserveBalanceUsd: string): ReserveCoverage => ({
  observedAtUnixMs,
  bankAsOfUnixMs: observedAtUnixMs,
  reserveAccountId: "reserve-rusd",
  currency: "USD",
  reserveBalanceMinor: "500000",
  reserveBalanceUsd,
  tokenSupplyRaw: "4000000000",
  liabilityUsd: "4000",
  surplusUsd: "1000",
  ratioPercent: 125,
  status: "covered",
})

describe("supply and reserve chart", () => {
  it("orders independent streams and carries their last known values forward", () => {
    const points = buildSupplyReservePoints(
      [token(1_000, "4000000000"), token(3_000, "4200000000")],
      [reserve(2_000, "5000.00")],
      token(3_000, "4200000000"),
      reserve(4_000, "4700.00"),
    )

    expect(points.map(({ observedAtUnixMs, supply, reserve: reserveValue }) =>
      ({ observedAtUnixMs, supply, reserve: reserveValue }))).toEqual([
      { observedAtUnixMs: 1_000, supply: 4000, reserve: undefined },
      { observedAtUnixMs: 2_000, supply: 4000, reserve: 5000 },
      { observedAtUnixMs: 3_000, supply: 4200, reserve: 5000 },
      { observedAtUnixMs: 4_000, supply: 4200, reserve: 4700 },
    ])
  })

  it("keeps the selected window and the newest observation in each sampling bucket", () => {
    const points = [0, 10, 20, 70, 130].map((seconds) => ({
      observedAtUnixMs: seconds * 1_000,
      time: String(seconds),
      supply: seconds,
    }))

    expect(selectChartWindow(points, {
      id: "medium",
      sampleIntervalMs: 60_000,
      rangeMs: 120_000,
    }).map((point) => point.observedAtUnixMs)).toEqual([20_000, 70_000, 130_000])
  })
})
