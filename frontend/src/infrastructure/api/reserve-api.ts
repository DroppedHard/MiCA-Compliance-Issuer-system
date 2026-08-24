import { Effect, Schema, pipe } from "effect"
import { ReserveCoverageSchema } from "@/domain/reserve"

export const fetchReserveCoveragePromise = () => Effect.runPromise(pipe(
  Effect.tryPromise({ try: async () => { const response = await fetch("/api/v1/reserves"); if (!response.ok) throw new Error(`Backend returned HTTP ${response.status}`); return response.json() as Promise<unknown> }, catch: (cause) => new Error(String(cause)) }),
  Effect.flatMap(Schema.decodeUnknown(ReserveCoverageSchema)),
  Effect.mapError((cause) => new Error(String(cause))),
))
