import { queryOptions } from "@tanstack/react-query"
import { fetchTokenObservationPromise } from "@/infrastructure/api/token-api"

export const tokenQueryKey = ["token", "latest"] as const
export const tokenHistoryQueryKey = ["token", "history"] as const

export const tokenQueryOptions = queryOptions({
  queryKey: tokenQueryKey,
  queryFn: fetchTokenObservationPromise,
  staleTime: 15_000,
  retry: 2,
})
