import { queryOptions } from "@tanstack/react-query"
import { fetchEsgHistoryPromise, fetchEsgObservationPromise } from "@/infrastructure/api/esg-api"

export const esgQueryKey = ["esg", "latest"] as const
export const esgQueryOptions = queryOptions({ queryKey: esgQueryKey, queryFn: fetchEsgObservationPromise, staleTime: 15_000, retry: 2 })
export const esgHistoryQueryKey = ["esg", "daily", 7] as const
export const esgHistoryQueryOptions = queryOptions({ queryKey: esgHistoryQueryKey, queryFn: fetchEsgHistoryPromise, staleTime: 15_000, retry: 2 })
