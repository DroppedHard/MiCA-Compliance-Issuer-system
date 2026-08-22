import { Schema } from "effect"

export const EsgEstimateSchema = Schema.Struct({
  dateUtc: Schema.String, status: Schema.String, dataOrigin: Schema.String, transactionCount: Schema.Number,
  energyLowerWh: Schema.Number, energyBestGuessWh: Schema.Number, energyUpperWh: Schema.Number, emissionsGCo2e: Schema.Number,
  renewableEnergyWh: Schema.Number, nuclearEnergyWh: Schema.Number, fossilEnergyWh: Schema.Number,
})

export const EsgObservationSchema = Schema.Struct({
  observedAtUnixMs: Schema.Number,
  lastProcessedBlock: Schema.Number,
  chainId: Schema.Number,
  contractAddress: Schema.String,
  currentDay: EsgEstimateSchema,
  methodology: Schema.Struct({
    version: Schema.String, annualTransactionsAssumption: Schema.Number,
    lowerEnergyWhPerTransaction: Schema.Number, bestGuessEnergyWhPerTransaction: Schema.Number, upperEnergyWhPerTransaction: Schema.Number,
    emissionsGCo2ePerTransaction: Schema.Number, renewablePercent: Schema.Number,
    nuclearPercent: Schema.Number, fossilPercent: Schema.Number,
    sourceName: Schema.String, sourceUrl: Schema.String, note: Schema.String,
  }),
})
export type EsgObservation = typeof EsgObservationSchema.Type

export const EsgHistorySchema = Schema.Struct({ days: Schema.Array(EsgEstimateSchema), methodology: EsgObservationSchema.fields.methodology })
export type EsgHistory = typeof EsgHistorySchema.Type
