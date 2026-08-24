import { queryOptions } from "@tanstack/react-query"
import { fetchReserveCoveragePromise } from "@/infrastructure/api/reserve-api"
export const reserveQueryKey = ["reserves", "latest"] as const
export const reserveHistoryQueryKey = ["reserves", "history"] as const
export const reserveQueryOptions = queryOptions({ queryKey: reserveQueryKey, queryFn: fetchReserveCoveragePromise, staleTime: 15_000, retry: 2 })
