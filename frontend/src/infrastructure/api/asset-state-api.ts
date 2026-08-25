import { Effect, Schema, pipe } from "effect"
import { AssetStateSchema } from "@/domain/asset-state"

export const fetchAssetStatePromise = () => Effect.runPromise(pipe(
  Effect.tryPromise({ try: async () => { const response = await fetch("/api/v1/asset-state"); if (!response.ok) throw new Error(`Backend returned HTTP ${response.status}`); return response.json() as Promise<unknown> }, catch: (cause) => new Error(String(cause)) }),
  Effect.flatMap(Schema.decodeUnknown(AssetStateSchema)),
))
