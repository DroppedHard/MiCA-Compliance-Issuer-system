import { Schema } from "effect"

export const ReserveCoverageSchema = Schema.Struct({
  observedAtUnixMs: Schema.Number, bankAsOfUnixMs: Schema.Number,
  reserveAccountId: Schema.String, currency: Schema.String,
  reserveBalanceMinor: Schema.String, reserveBalanceUsd: Schema.String,
  tokenSupplyRaw: Schema.String, liabilityUsd: Schema.String,
  surplusUsd: Schema.String, ratioPercent: Schema.NullOr(Schema.Number),
  status: Schema.Literal("covered", "undercollateralized"),
})
export type ReserveCoverage = typeof ReserveCoverageSchema.Type

export type SupplyReservePoint = {
  observedAtUnixMs: number
  time: string
  supply?: number
  reserve?: number
}

export const toReserveValue = (coverage: ReserveCoverage): number =>
  Number(coverage.reserveBalanceUsd)
