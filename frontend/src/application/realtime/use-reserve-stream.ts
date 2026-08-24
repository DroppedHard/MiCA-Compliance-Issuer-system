import { useEffect } from "react"
import { useQueryClient } from "@tanstack/react-query"
import { Effect, Schema, pipe } from "effect"
import { ReserveCoverageSchema, type ReserveCoverage } from "@/domain/reserve"
import { reserveHistoryQueryKey, reserveQueryKey } from "@/application/queries/reserve-query"

const MAX_VISIBLE_OBSERVATIONS = 360

export const appendReserveObservation = (
  current: ReserveCoverage[],
  observation: ReserveCoverage,
  limit = MAX_VISIBLE_OBSERVATIONS,
): ReserveCoverage[] => [...current, observation].slice(-limit)

export const decodeReserveEventData = (data: string) => Effect.runPromise(pipe(Effect.try({ try: () => JSON.parse(data) as unknown, catch: () => undefined }), Effect.flatMap(Schema.decodeUnknown(ReserveCoverageSchema))))
export function useReserveStream() {
  const queryClient = useQueryClient()
  useEffect(() => {
    const source = new EventSource("/api/v1/reserves/stream")
    source.addEventListener("reserve", (event) => {
      decodeReserveEventData(event.data).then((value) => {
        queryClient.setQueryData(reserveQueryKey, value)
        queryClient.setQueryData<ReserveCoverage[]>(reserveHistoryQueryKey, (current = []) =>
          appendReserveObservation(current, value),
        )
      }).catch(() => undefined)
    })
    return () => source.close()
  }, [queryClient])
}
