import { Effect, Schema, pipe } from "effect"
import { ReserveCoverageSchema } from "@/domain/reserve"

export const fetchReserveCoveragePromise = () => Effect.runPromise(pipe(
  Effect.tryPromise({ try: async () => { const response = await fetch("/api/v1/reserves"); if (!response.ok) throw new Error(`Backend returned HTTP ${response.status}`); return response.json() as Promise<unknown> }, catch: (cause) => new Error(String(cause)) }),
  Effect.flatMap(Schema.decodeUnknown(ReserveCoverageSchema)),
  Effect.mapError((cause) => new Error(String(cause))),
))

export type ReserveAdjustmentDirection = "deposit" | "withdrawal"

export const adjustReservePromise = (
  operationId: string,
  direction: ReserveAdjustmentDirection,
  amountUsd: string,
  reason: string,
) => Effect.runPromise(Effect.tryPromise({
  try: async () => {
    const response = await fetch("/api/v1/admin/reserves/adjustments", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ operationId, direction, amountUsd, reason }),
    })
    const body = await response.json() as { error?: string }
    if (!response.ok) throw new Error(body.error ?? `Backend returned HTTP ${response.status}`)
    return body
  },
  catch: (cause) => cause instanceof Error ? cause : new Error(String(cause)),
}))
