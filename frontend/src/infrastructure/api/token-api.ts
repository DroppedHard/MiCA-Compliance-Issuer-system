import { Effect, Schema, pipe } from "effect"
import { TokenObservationSchema } from "@/domain/token"

export class TokenApiError extends Error {
  readonly _tag = "TokenApiError"
}

export const fetchTokenObservation = () =>
  pipe(
    Effect.tryPromise({
      try: async () => {
        const response = await fetch("/api/v1/token")
        if (!response.ok) throw new Error(`Backend returned HTTP ${response.status}`)
        return response.json() as Promise<unknown>
      },
      catch: (cause) => new TokenApiError(String(cause)),
    }),
    Effect.flatMap(Schema.decodeUnknown(TokenObservationSchema)),
    Effect.mapError((cause) => new TokenApiError(String(cause))),
  )

export const fetchTokenObservationPromise = () =>
  Effect.runPromise(fetchTokenObservation())
