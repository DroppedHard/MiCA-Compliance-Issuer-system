import { Effect, Schema } from "effect"

const AddressRestriction = Schema.Struct({
  address: Schema.String,
  reason: Schema.String,
  active: Schema.Boolean,
  transactionHash: Schema.NullOr(Schema.String),
  updatedAtUnixMs: Schema.Number,
})
export type AddressRestriction = typeof AddressRestriction.Type

const decode = async <A>(response: Response, schema: Schema.Schema<A>): Promise<A> => {
  if (!response.ok) {
    const body = await response.json().catch(() => null) as { error?: string } | null
    throw new Error(body?.error ?? `Błąd HTTP ${response.status}`)
  }
  return Effect.runPromise(Schema.decodeUnknown(schema)(await response.json()))
}

export const addressBlacklistApi = {
  list: () => fetch("/api/v1/admin/address-blacklist").then(response => decode(response, Schema.Array(AddressRestriction))),
  add: (address: string, reason: string) => fetch("/api/v1/admin/address-blacklist", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ address, reason }),
  }).then(response => decode(response, AddressRestriction)),
  remove: (address: string) => fetch(`/api/v1/admin/address-blacklist/${encodeURIComponent(address)}`, { method: "DELETE" })
    .then(response => decode(response, AddressRestriction)),
}
