import { useEffect } from "react"
import { useQueryClient } from "@tanstack/react-query"
import { Effect, Schema, pipe } from "effect"
import { EsgObservationSchema } from "@/domain/esg"
import { esgHistoryQueryKey, esgQueryKey } from "@/application/queries/esg-query"
import type { EsgHistory, EsgObservation } from "@/domain/esg"

export const decodeEsgEventData = (data: string) => {
  const decoded = pipe(
    Effect.try({ try: () => JSON.parse(data) as unknown, catch: () => undefined }),
    Effect.flatMap(Schema.decodeUnknown(EsgObservationSchema)),
  )
  return Effect.runPromise(decoded)
}

export const mergeEsgHistory = (history: EsgHistory | undefined, observation: EsgObservation): EsgHistory | undefined =>
  history ? {
    ...history,
    days: [...history.days.filter((day) => day.dateUtc !== observation.currentDay.dateUtc), observation.currentDay]
      .sort((left, right) => left.dateUtc.localeCompare(right.dateUtc))
      .slice(-7),
  } : history

export function useEsgStream() {
  const queryClient = useQueryClient()
  useEffect(() => {
    const source = new EventSource("/api/v1/esg/stream")
    source.addEventListener("esg", (event) => {
      decodeEsgEventData(event.data).then((value) => {
        queryClient.setQueryData(esgQueryKey, value)
        queryClient.setQueryData<EsgHistory>(esgHistoryQueryKey, (history) => mergeEsgHistory(history, value))
      }).catch(() => undefined)
    })
    return () => source.close()
  }, [queryClient])
}
