import { useState } from "react"
import { useMutation, useQueryClient } from "@tanstack/react-query"
import { TriangleAlert } from "lucide-react"
import { assetStateQueryKey } from "@/application/queries/asset-state-query"
import { Dialog } from "@/components/ui/dialog"
import type { AssetState } from "@/domain/asset-state"
import { enterWindDownPromise } from "@/infrastructure/api/asset-state-api"

export function WindDownControl({ state }: { state?: AssetState["state"] }) {
  const [open, setOpen] = useState(false)
  const [reason, setReason] = useState("")
  const queryClient = useQueryClient()
  const mutation = useMutation({
    mutationFn: () => enterWindDownPromise(crypto.randomUUID(), reason.trim()),
    onSuccess: (value) => {
      queryClient.setQueryData(assetStateQueryKey, value)
      setOpen(false)
    },
  })

  if (state === "wind_down") {
    return <p className="mt-3 border-t border-current/20 pt-3 font-medium">Wygaszanie jest aktywne w łańcuchu bloków. Emisja i zwykłe transfery są zablokowane, a wykupy pozostają dostępne.</p>
  }

  return <>
    <button
      type="button"
      className="mt-3 w-full rounded-lg border border-rose-400/40 bg-rose-500/10 px-3 py-2 font-medium text-rose-200 transition hover:bg-rose-500/20"
      onClick={() => setOpen(true)}
    >
      Rozpocznij wygaszanie tokenu
    </button>
    <Dialog open={open} onClose={() => !mutation.isPending && setOpen(false)} title="Nieodwracalne wygaszanie tokenu">
      <div className="space-y-4 text-sm text-slate-300">
        <div className="flex gap-3 rounded-xl border border-rose-500/30 bg-rose-500/10 p-4 text-rose-200">
          <TriangleAlert className="mt-0.5 size-5 shrink-0" />
          <p>Po potwierdzeniu kontrakt trwale zablokuje mint i zwykłe transfery. Nadal będzie pozwalał emitentowi spalać tokeny w procesie wykupu.</p>
        </div>
        <label className="block">
          <span className="mb-2 block font-medium text-slate-200">Powód decyzji</span>
          <textarea
            className="min-h-28 w-full rounded-xl border border-slate-700 bg-slate-950 p-3 text-white outline-none focus:border-rose-400"
            maxLength={500}
            value={reason}
            onChange={(event) => setReason(event.target.value)}
            placeholder="Np. decyzja o zakończeniu działalności emitenta"
          />
        </label>
        {mutation.isError && <p className="text-rose-300">{mutation.error.message}</p>}
        <div className="flex justify-end gap-3">
          <button className="rounded-lg border border-slate-700 px-4 py-2 text-slate-300" onClick={() => setOpen(false)} disabled={mutation.isPending}>Anuluj</button>
          <button
            className="rounded-lg bg-rose-500 px-4 py-2 font-semibold text-white disabled:cursor-not-allowed disabled:opacity-50"
            disabled={!reason.trim() || mutation.isPending}
            onClick={() => mutation.mutate()}
          >
            {mutation.isPending ? "Oczekiwanie na łańcuch bloków…" : "Potwierdź wygaszanie"}
          </button>
        </div>
      </div>
    </Dialog>
  </>
}
