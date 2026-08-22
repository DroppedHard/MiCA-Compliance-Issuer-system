import { Effect, Schema, pipe } from "effect"
import { EsgHistorySchema, EsgObservationSchema } from "@/domain/esg"

export const fetchEsgObservation = () => pipe(
  Effect.tryPromise({
    try: async () => {
      const response = await fetch("/api/v1/esg")
      if (!response.ok) throw new Error(`Backend returned HTTP ${response.status}`)
      return response.json() as Promise<unknown>
    },
    catch: (cause) => new Error(String(cause)),
  }),
  Effect.flatMap(Schema.decodeUnknown(EsgObservationSchema)),
  Effect.mapError((cause) => new Error(String(cause))),
)

export const fetchEsgObservationPromise = () => Effect.runPromise(fetchEsgObservation())

export const fetchEsgHistoryPromise = () => Effect.runPromise(pipe(
  Effect.tryPromise({ try: async () => { const response = await fetch("/api/v1/esg/daily"); if (!response.ok) throw new Error(`Backend returned HTTP ${response.status}`); return response.json() as Promise<unknown> }, catch: (cause) => new Error(String(cause)) }),
  Effect.flatMap(Schema.decodeUnknown(EsgHistorySchema)),
  Effect.mapError((cause) => new Error(String(cause))),
))
