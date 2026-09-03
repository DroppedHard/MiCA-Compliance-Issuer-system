import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { Ban, Trash2 } from "lucide-react"
import { useState } from "react"
import { addressBlacklistApi } from "@/infrastructure/api/address-blacklist-api"

export function AddressBlacklistControl() {
  const client = useQueryClient()
  const [address, setAddress] = useState("")
  const [reason, setReason] = useState("Decyzja administratora emitenta")
  const entries = useQuery({ queryKey: ["issuer", "address-blacklist"], queryFn: addressBlacklistApi.list })
  const refresh = () => client.invalidateQueries({ queryKey: ["issuer", "address-blacklist"] })
  const add = useMutation({ mutationFn: () => addressBlacklistApi.add(address, reason), onSuccess: async () => { setAddress(""); await refresh() } })
  const remove = useMutation({ mutationFn: addressBlacklistApi.remove, onSuccess: refresh })

  return <div className="address-blacklist-control">
    <div className="flex items-start gap-2"><Ban className="mt-0.5 size-4 text-rose-300"/><div><p className="text-sm font-medium text-slate-100">Czarna lista adresów</p><p className="text-xs text-slate-400">Zmiana jest zapisywana w bazie emitenta i wykonywana w kontrakcie rUSD.</p></div></div>
    <form onSubmit={event => { event.preventDefault(); add.mutate() }}>
      <label>Adres portfela<input value={address} onChange={event => setAddress(event.target.value)} placeholder="0x…" spellCheck={false}/></label>
      <label>Uzasadnienie<input value={reason} onChange={event => setReason(event.target.value)}/></label>
      <button disabled={add.isPending || !address.trim() || !reason.trim()}>Dodaj do czarnej listy</button>
    </form>
    {add.error && <p className="text-xs text-rose-300">{add.error.message}</p>}
    <div className="address-blacklist-items">{entries.data?.filter(entry => entry.active).map(entry => <article key={entry.address}><div><strong>{entry.address}</strong><span>{entry.reason}</span></div><button title="Usuń blokadę" onClick={() => remove.mutate(entry.address)}><Trash2/></button></article>)}{entries.data?.filter(entry => entry.active).length === 0 && <p>Brak zablokowanych adresów.</p>}</div>
  </div>
}
