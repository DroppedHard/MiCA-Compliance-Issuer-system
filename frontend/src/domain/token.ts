import { Schema, pipe } from "effect"

export const TokenSnapshotSchema = Schema.Struct({
  chainId: Schema.Number,
  blockNumber: Schema.Number,
  contractAddress: Schema.String,
  name: Schema.String,
  symbol: Schema.String,
  decimals: Schema.Number,
  totalSupplyRaw: Schema.String,
})

export const TokenObservationSchema = Schema.Struct({
  observedAtUnixMs: Schema.Number,
  snapshot: TokenSnapshotSchema,
})

export type TokenObservation = typeof TokenObservationSchema.Type

export const formatTokenAmount = (raw: string, decimals: number): string =>
  pipe(
    raw.padStart(decimals + 1, "0"),
    (value) => [value.slice(0, -decimals) || "0", value.slice(-decimals)] as const,
    ([whole, fraction]) => `${whole}.${fraction}`,
    (value) => value.replace(/\.0+$|(?<=\.[0-9]*?)0+$/, ""),
  )

export const shortenAddress = (address: string): string =>
  pipe(address, (value) => `${value.slice(0, 8)}…${value.slice(-6)}`)

export const toChartPoint = (observation: TokenObservation) => ({
  time: new Date(observation.observedAtUnixMs).toLocaleTimeString("pl-PL", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }),
  supply: Number(formatTokenAmount(observation.snapshot.totalSupplyRaw, observation.snapshot.decimals)),
})
