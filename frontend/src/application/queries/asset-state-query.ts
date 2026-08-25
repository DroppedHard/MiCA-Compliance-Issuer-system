import { queryOptions } from "@tanstack/react-query"
import { fetchAssetStatePromise } from "@/infrastructure/api/asset-state-api"

export const assetStateQueryKey = ["asset-state", "current"] as const
export const assetStateQueryOptions = queryOptions({ queryKey: assetStateQueryKey, queryFn: fetchAssetStatePromise, staleTime: 15_000, retry: 2 })
