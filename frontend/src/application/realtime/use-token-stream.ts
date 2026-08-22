import { useEffect, useState } from "react"
import { useQueryClient } from "@tanstack/react-query"
import type { TokenObservation } from "@/domain/token"
import { tokenHistoryQueryKey, tokenQueryKey } from "@/application/queries/token-query"
import { openTokenStream, type StreamConnection } from "@/infrastructure/realtime/token-event-source"

const MAX_VISIBLE_OBSERVATIONS = 360

export function useTokenStream() {
  const queryClient = useQueryClient()
  const [connection, setConnection] = useState<StreamConnection>("connecting")

  useEffect(() => openTokenStream({
    onConnectionChange: setConnection,
    onObservation: (observation) => {
      queryClient.setQueryData(tokenQueryKey, observation)
      queryClient.setQueryData<TokenObservation[]>(tokenHistoryQueryKey, (current = []) =>
        [...current, observation].slice(-MAX_VISIBLE_OBSERVATIONS),
      )
    },
  }), [queryClient])

  return connection
}
