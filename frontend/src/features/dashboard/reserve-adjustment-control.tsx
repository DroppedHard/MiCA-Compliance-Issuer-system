import { useState } from "react"
import { useMutation, useQueryClient } from "@tanstack/react-query"
import { ArrowDownToLine, ArrowUpFromLine } from "lucide-react"
import { reserveQueryKey } from "@/application/queries/reserve-query"
import { adjustReservePromise, type ReserveAdjustmentDirection } from "@/infrastructure/api/reserve-api"

export function ReserveAdjustmentControl() {
  const [direction, setDirection] = useState<ReserveAdjustmentDirection>("deposit")
  const [amountUsd, setAmountUsd] = useState("1000.00")
  const [reason, setReason] = useState("Demonstracyjna korekta rezerwy")
  const queryClient = useQueryClient()
  const mutation = useMutation({
    mutationFn: () => adjustReservePromise(crypto.randomUUID(), direction, amountUsd.trim(), reason.trim()),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: reserveQueryKey }),
  })

  return <div className="mt-3 border-t border-slate-800 pt-3">
    <div className="mb-3">
      <p className="font-medium text-slate-200">Testowa korekta rezerwy</p>
      <p className="mt-1 text-xs leading-5 text-slate-500">Operacja zmienia wyłącznie saldo USD w mockBanku. Nie emituje ani nie spala rUSD.</p>
    </div>
    <div className="flex flex-col gap-2">
      <div className="flex self-start rounded-lg border border-slate-700 p-1">
        <DirectionButton active={direction === "deposit"} onClick={() => setDirection("deposit")} label="Dodaj" icon={ArrowDownToLine} />
        <DirectionButton active={direction === "withdrawal"} onClick={() => setDirection("withdrawal")} label="Odejmij" icon={ArrowUpFromLine} />
      </div>
      <label className="block">
        <span className="sr-only">Kwota w USD</span>
        <input className="h-9 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 text-sm text-white outline-none focus:border-teal-400" inputMode="decimal" value={amountUsd} onChange={(event) => setAmountUsd(event.target.value)} placeholder="Kwota USD" />
      </label>
      <label className="block">
        <span className="sr-only">Powód korekty</span>
        <input className="h-9 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 text-sm text-white outline-none focus:border-teal-400" maxLength={500} value={reason} onChange={(event) => setReason(event.target.value)} placeholder="Powód korekty" />
      </label>
      <button className="h-9 rounded-lg bg-teal-400 px-4 text-sm font-semibold text-slate-950 disabled:cursor-not-allowed disabled:opacity-50" disabled={!amountUsd.trim() || !reason.trim() || mutation.isPending} onClick={() => mutation.mutate()}>
        {mutation.isPending ? "Zapisywanie…" : "Wykonaj"}
      </button>
    </div>
    {mutation.isSuccess && <p className="mt-3 text-xs text-emerald-300">Korekta została zapisana w mockBanku. Wskaźnik odświeży się najpóźniej po kolejnym pollingu.</p>}
    {mutation.isError && <p className="mt-3 text-xs text-rose-300">{mutation.error.message}</p>}
  </div>
}

function DirectionButton({ active, onClick, label, icon: Icon }: { active: boolean; onClick: () => void; label: string; icon: typeof ArrowDownToLine }) {
  return <button type="button" className={`flex h-9 items-center gap-1.5 rounded-md px-3 text-xs font-medium transition ${active ? "bg-teal-400/15 text-teal-300" : "text-slate-400 hover:text-slate-200"}`} onClick={onClick}><Icon className="size-3.5" />{label}</button>
}
