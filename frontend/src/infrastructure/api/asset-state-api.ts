import { Effect, Schema, pipe } from "effect"
import { AssetStateSchema } from "@/domain/asset-state"

export const fetchAssetStatePromise = () => Effect.runPromise(pipe(
  Effect.tryPromise({ try: async () => { const response = await fetch("/api/v1/asset-state"); if (!response.ok) throw new Error(`Backend returned HTTP ${response.status}`); return response.json() as Promise<unknown> }, catch: (cause) => new Error(String(cause)) }),
  Effect.flatMap(Schema.decodeUnknown(AssetStateSchema)),
))

export const enterWindDownPromise = (operationId: string, reason: string) => Effect.runPromise(pipe(
  Effect.tryPromise({
    try: async () => {
      const response = await fetch("/api/v1/admin/asset-state/wind-down", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ operationId, reason }),
      })
      if (!response.ok) {
        const body = await response.json().catch(() => null) as { error?: string } | null
        throw new Error(body?.error ?? `Backend returned HTTP ${response.status}`)
      }
      return response.json() as Promise<unknown>
    },
    catch: (cause) => cause instanceof Error ? cause : new Error(String(cause)),
  }),
  Effect.flatMap(Schema.decodeUnknown(AssetStateSchema)),
))
