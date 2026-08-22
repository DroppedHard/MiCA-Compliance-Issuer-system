import { Effect, Schema, pipe } from "effect"
import { TokenObservationSchema, type TokenObservation } from "@/domain/token"

export type StreamConnection = "connecting" | "live" | "disconnected"

interface TokenStreamHandlers {
  onObservation: (observation: TokenObservation) => void
  onConnectionChange: (status: StreamConnection) => void
}

export function openTokenStream(handlers: TokenStreamHandlers): () => void {
  handlers.onConnectionChange("connecting")
  const source = new EventSource("/api/v1/token/stream")

  source.onopen = () => handlers.onConnectionChange("live")
  source.onerror = () => handlers.onConnectionChange("disconnected")
  source.addEventListener("token", (event) => {
    const decode = pipe(
      Effect.try({
        try: () => JSON.parse(event.data) as unknown,
        catch: () => undefined,
      }),
      Effect.flatMap(Schema.decodeUnknown(TokenObservationSchema)),
    )
    Effect.runPromise(decode)
      .then(handlers.onObservation)
      .catch(() => undefined)
  })

  return () => source.close()
}
