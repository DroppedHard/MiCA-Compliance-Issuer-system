import { useEffect } from "react"
import { useQueryClient } from "@tanstack/react-query"
import { Effect, Schema, pipe } from "effect"
import { AssetStateSchema } from "@/domain/asset-state"
import { assetStateQueryKey } from "@/application/queries/asset-state-query"

export const decodeAssetStateEventData = (data: string) => Effect.runPromise(pipe(
  Effect.try({ try: () => JSON.parse(data) as unknown, catch: () => undefined }),
  Effect.flatMap(Schema.decodeUnknown(AssetStateSchema)),
))

export function useAssetStateStream() {
  const queryClient = useQueryClient()
  useEffect(() => {
    const source = new EventSource("/api/v1/asset-state/stream")
    source.addEventListener("asset-state", (event) => decodeAssetStateEventData(event.data).then((value) => queryClient.setQueryData(assetStateQueryKey, value)).catch(() => undefined))
    return () => source.close()
  }, [queryClient])
}
