import { Schema } from "effect"

export const AssetStateSchema = Schema.Struct({
  state: Schema.Literal("active", "warning", "mint_blocked", "data_unavailable", "wind_down"),
  reason: Schema.String,
  reserveCoveragePercent: Schema.NullOr(Schema.Number),
  evidenceAtUnixMs: Schema.NullOr(Schema.Number),
  policyVersion: Schema.String,
  updatedAtUnixMs: Schema.Number,
})

export type AssetState = typeof AssetStateSchema.Type

export const assetStateLabel = (state: AssetState["state"]): string => ({
  active: "Aktywny",
  warning: "Ostrzeżenie",
  mint_blocked: "Emisja zablokowana",
  data_unavailable: "Brak danych",
  wind_down: "Wygaszanie",
})[state]
