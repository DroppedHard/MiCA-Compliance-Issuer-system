import { useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { Activity, Blocks, Building2, CircleDollarSign, Database, ExternalLink, Info, Leaf, Radio, ShieldCheck, TriangleAlert, Zap } from "lucide-react"
import { reserveQueryOptions } from "@/application/queries/reserve-query"
import { useReserveStream } from "@/application/realtime/use-reserve-stream"
import { esgHistoryQueryOptions, esgQueryOptions } from "@/application/queries/esg-query"
import { useEsgStream } from "@/application/realtime/use-esg-stream"
import { tokenQueryOptions } from "@/application/queries/token-query"
import { useTokenStream } from "@/application/realtime/use-token-stream"
import { Badge } from "@/components/ui/badge"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Skeleton } from "@/components/ui/skeleton"
import { Dialog } from "@/components/ui/dialog"
import { formatTokenAmount, shortenAddress } from "@/domain/token"
import { SupplyChart } from "./supply-chart"
import { EsgEnergyChart } from "./esg-energy-chart"

export function AdminDashboard() {
  const token = useQuery(tokenQueryOptions)
  const esg = useQuery(esgQueryOptions)
  const esgHistory = useQuery(esgHistoryQueryOptions)
  const reserve = useQuery(reserveQueryOptions)
  const [detailsOpen, setDetailsOpen] = useState(false)
  const connection = useTokenStream()
  useEsgStream()
  useReserveStream()

  if (token.isPending) return <DashboardLoading />

  if (token.isError) {
    return (
      <main className="grid min-h-screen place-items-center p-6">
        <Card className="max-w-lg border-rose-500/30">
          <CardHeader><CardTitle>Backend jest niedostępny</CardTitle></CardHeader>
          <CardContent className="space-y-3 text-sm text-slate-400">
            <p>Szczegóły techniczne: {token.error.message}</p>
            <p>Uruchom lokalny blockchain, wdroż token i włącz backend w Rust. Panel automatycznie ponowi próbę.</p>
          </CardContent>
        </Card>
      </main>
    )
  }

  const { snapshot, observedAtUnixMs } = token.data
  const supply = formatTokenAmount(snapshot.totalSupplyRaw, snapshot.decimals)

  return (
    <main className="mx-auto min-h-screen max-w-7xl p-5 md:p-8">
      <header className="mb-8 flex flex-col gap-5 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="mb-2 font-mono text-xs uppercase tracking-[0.28em] text-teal-400">Środowisko badawcze</p>
          <h1 className="text-3xl font-semibold tracking-tight text-white md:text-4xl">Panel zarządzania EMT</h1>
          <p className="mt-2 max-w-2xl text-slate-400">Bieżący podgląd lokalnie wdrożonego kryptoaktywa w trybie tylko do odczytu.</p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <a href="/client" className="inline-flex items-center gap-2 rounded-lg border border-slate-700 px-3 py-2 text-xs font-medium text-slate-300 transition hover:border-teal-400/40 hover:bg-teal-400/10 hover:text-teal-200">Widok klienta <ExternalLink className="size-3.5" /></a>
          <Badge className={connection === "live" ? "border-teal-400/30 bg-teal-400/10 text-teal-300" : "border-amber-400/30 bg-amber-400/10 text-amber-300"}>
            <Radio className="mr-1.5 size-3" /> SSE {connectionLabel(connection)}
          </Badge>
        </div>
      </header>

      <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <Metric icon={CircleDollarSign} label="Całkowita podaż" value={`${supply} ${snapshot.symbol}`} detail={`${snapshot.decimals} miejsc dziesiętnych`} />
        <Metric icon={Blocks} label="Najnowszy blok" value={snapshot.blockNumber.toLocaleString("pl-PL")} detail={`Identyfikator sieci ${snapshot.chainId}`} />
        <Metric icon={Database} label="Kontrakt" value={shortenAddress(snapshot.contractAddress)} detail={snapshot.name} mono />
        <Metric icon={Activity} label="Czas obserwacji" value={new Date(observedAtUnixMs).toLocaleTimeString("pl-PL")} detail={new Date(observedAtUnixMs).toLocaleDateString("pl-PL")} />
      </section>

      <section className="mt-4 grid gap-4 lg:grid-cols-[2fr_1fr]">
        <Card>
          <CardHeader><CardTitle>Podaż i rezerwa</CardTitle><CardDescription>Ostatnie obserwacje podaży tokenu i salda mockBanku odebrane przez SSE.</CardDescription></CardHeader>
          <CardContent><SupplyChart latestToken={token.data} latestReserve={reserve.data} /></CardContent>
        </Card>
        <Card>
          <CardHeader><CardTitle>Stan systemu</CardTitle><CardDescription>Bieżący stan połączeń między warstwami.</CardDescription></CardHeader>
          <CardContent className="space-y-4">
            <StatusRow label="Ostatni odczyt backendu" value={new Date(observedAtUnixMs).toLocaleTimeString("pl-PL")} />
            <StatusRow label="Cache backendu" value="Pamięć procesu · 24 godz." />
            <StatusRow label="Transport" value="SSE + inicjalizacja HTTP" />
            <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4 text-xs leading-6 text-slate-400">
              <ShieldCheck className="mb-2 size-5 text-teal-400" />
              Panel działa tylko do odczytu. Operacje administracyjne tokenu nie są jeszcze podłączone.
            </div>
          </CardContent>
        </Card>
      </section>
      <section className="mt-4">
        <Card className={reserve.data?.status === "undercollateralized" ? "border-rose-500/40" : undefined}>
          <CardHeader><CardTitle>Pokrycie rezerwy rUSD</CardTitle><CardDescription>Dane z zewnętrznego mockBanku porównane z aktualną podażą tokenu.</CardDescription></CardHeader>
          <CardContent>
            {reserve.isPending && <Skeleton className="h-28" />}
            {reserve.isError && <div className="flex items-center gap-3 rounded-xl border border-amber-500/30 bg-amber-500/10 p-4 text-sm text-amber-200"><TriangleAlert className="size-5" /> Brak świeżych danych z mockBanku. Uruchom serwis na porcie 3100.</div>}
            {reserve.data && <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
              <EsgMetric icon={Building2} label="Rezerwa bankowa" value={`${reserve.data.reserveBalanceUsd} USD`} />
              <EsgMetric icon={CircleDollarSign} label="Zobowiązanie tokenu" value={`${reserve.data.liabilityUsd} USD`} />
              <EsgMetric icon={ShieldCheck} label="Wskaźnik pokrycia" value={reserve.data.ratioPercent === null ? "Brak podaży" : `${reserve.data.ratioPercent.toLocaleString("pl-PL", { maximumFractionDigits: 2 })}%`} />
              <EsgMetric icon={reserve.data.status === "covered" ? ShieldCheck : TriangleAlert} label={Number(reserve.data.surplusUsd) >= 0 ? "Nadwyżka" : "Niedobór"} value={`${reserve.data.surplusUsd} USD`} />
            </div>}
            {reserve.data && <div className={`mt-4 rounded-xl border p-3 text-sm ${reserve.data.status === "covered" ? "border-emerald-500/30 bg-emerald-500/10 text-emerald-300" : "border-rose-500/30 bg-rose-500/10 text-rose-300"}`}>{reserve.data.status === "covered" ? "Rezerwa w pełni pokrywa aktualną podaż rUSD." : "Rezerwa nie pokrywa aktualnej podaży rUSD. Jest to sygnał monitorujący — token nie jest jeszcze automatycznie blokowany."}</div>}
          </CardContent>
        </Card>
      </section>
      {esg.data && <section className="mt-4">
        <Card>
          <CardHeader className="flex-row items-start justify-between gap-4">
            <div><CardTitle>Dzienne estymaty środowiskowe</CardTitle><CardDescription>Dane prowizoryczne dla {esg.data.currentDay.dateUtc}, aktualizowane wraz z obserwacją blockchaina.</CardDescription></div>
            <button className="flex shrink-0 items-center gap-2 rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:bg-slate-800" onClick={() => setDetailsOpen(true)}><Info className="size-4" /> Jak obliczono?</button>
          </CardHeader>
          <CardContent>
            <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
              <EsgMetric icon={Activity} label="Transakcje tokenu" value={esg.data.currentDay.transactionCount.toLocaleString("pl-PL")} />
              <EsgMetric icon={Zap} label="Najlepsza estymata" value={`${esg.data.currentDay.energyBestGuessWh.toLocaleString("pl-PL", { maximumFractionDigits: 3 })} Wh`} />
              <EsgMetric icon={Leaf} label="Emisje" value={`${esg.data.currentDay.emissionsGCo2e.toLocaleString("pl-PL", { maximumFractionDigits: 3 })} g CO₂e`} />
              <EsgMetric icon={Blocks} label="Przetworzony blok" value={esg.data.lastProcessedBlock.toLocaleString("pl-PL")} />
            </div>
            <div className="mt-5 grid gap-3 md:grid-cols-3">
              <EnergyPart label="OZE" percent={esg.data.methodology.renewablePercent} value={esg.data.currentDay.renewableEnergyWh} color="bg-emerald-400" />
              <EnergyPart label="Energia jądrowa" percent={esg.data.methodology.nuclearPercent} value={esg.data.currentDay.nuclearEnergyWh} color="bg-violet-400" />
              <EnergyPart label="Paliwa kopalne" percent={esg.data.methodology.fossilPercent} value={esg.data.currentDay.fossilEnergyWh} color="bg-amber-400" />
            </div>
            {esgHistory.data && <div className="mt-8 border-t border-slate-800 pt-6">
              <div className="mb-4 flex items-center justify-between"><div><p className="font-medium text-slate-200">Ostatnie 7 dni</p><p className="text-xs text-slate-500">Linia: najlepsza estymata · pasmo: dolny i górny scenariusz Cambridge</p></div><Badge className="border-slate-700 bg-slate-800 text-slate-300">Wh</Badge></div>
              <EsgEnergyChart days={esgHistory.data.days} />
            </div>}
          </CardContent>
        </Card>
        <Dialog open={detailsOpen} onClose={() => setDetailsOpen(false)} title="Jak obliczono estymatę?">
          <div className="space-y-4 text-sm leading-6 text-slate-300">
            <p>{esg.data.methodology.note}</p>
            <p>Cambridge modeluje całą sieć Ethereum PoS w trzech scenariuszach rocznych: <strong>1,26 GWh</strong> (dolny), <strong>7,87 GWh</strong> (best guess) i <strong>11,49 GWh</strong> (górny). Granice opisują różne profile sprzętu i hostingu; nie są statystycznym przedziałem ufności.</p>
            <p>System dzieli każdy scenariusz przez założone <strong>{esg.data.methodology.annualTransactionsAssumption.toLocaleString("pl-PL")}</strong> transakcji Ethereum rocznie. Daje to odpowiednio <strong>{esg.data.methodology.lowerEnergyWhPerTransaction} Wh</strong>, <strong>{esg.data.methodology.bestGuessEnergyWhPerTransaction} Wh</strong> i <strong>{esg.data.methodology.upperEnergyWhPerTransaction} Wh</strong> na transakcję tokenu.</p>
            <p>Każda unikalna transakcja emitująca <code className="text-teal-300">Transfer</code> jest mnożona przez wszystkie trzy współczynniki. Jest to demonstracyjna alokacja wpływu sieci, a nie pomiar energii wywołanej konkretnym transferem.</p>
            <p>Miks: {esg.data.methodology.renewablePercent}% OZE, {esg.data.methodology.nuclearPercent}% energii jądrowej i {esg.data.methodology.fossilPercent}% paliw kopalnych. Niewielka różnica do 100% wynika z zaokrągleń danych źródłowych.</p>
            <a className="inline-flex text-teal-300 underline decoration-teal-500/40 underline-offset-4" href={esg.data.methodology.sourceUrl} target="_blank" rel="noreferrer">{esg.data.methodology.sourceName}</a>
            <p className="text-xs text-slate-500">Wersja metodologii: {esg.data.methodology.version}</p>
          </div>
        </Dialog>
      </section>}
    </main>
  )
}

function EsgMetric({ icon: Icon, label, value }: { icon: typeof Activity; label: string; value: string }) { return <div className="rounded-xl border border-slate-800 bg-slate-950/60 p-4"><Icon className="mb-3 size-4 text-teal-400" /><p className="text-xs text-slate-500">{label}</p><p className="mt-1 text-xl font-semibold text-white">{value}</p></div> }
function EnergyPart({ label, percent, value, color }: { label: string; percent: number; value: number; color: string }) { return <div><div className="mb-2 flex justify-between text-xs"><span className="text-slate-400">{label}</span><span className="text-slate-200">{percent}% · {value.toLocaleString("pl-PL", { maximumFractionDigits: 3 })} Wh</span></div><div className="h-2 overflow-hidden rounded-full bg-slate-800"><div className={`h-full ${color}`} style={{ width: `${percent}%` }} /></div></div> }

function Metric({ icon: Icon, label, value, detail, mono = false }: { icon: typeof Activity; label: string; value: string; detail: string; mono?: boolean }) {
  return <Card><CardHeader className="flex-row items-center justify-between pb-3"><CardDescription>{label}</CardDescription><Icon className="size-4 text-teal-400" /></CardHeader><CardContent><p className={`truncate text-2xl font-semibold text-white ${mono ? "font-mono text-xl" : ""}`}>{value}</p><p className="mt-1 truncate text-xs text-slate-500">{detail}</p></CardContent></Card>
}

function StatusRow({ label, value }: { label: string; value: string }) {
  return <div className="flex items-center justify-between border-b border-slate-800 pb-3 text-sm"><span className="text-slate-400">{label}</span><span className="font-medium text-slate-200">{value}</span></div>
}

function DashboardLoading() {
  return <main className="mx-auto min-h-screen max-w-7xl p-8"><Skeleton className="mb-8 h-20 w-96 max-w-full" /><div className="grid gap-4 md:grid-cols-4">{Array.from({ length: 4 }).map((_, index) => <Skeleton key={index} className="h-36" />)}</div><Skeleton className="mt-4 h-96" /></main>
}

function connectionLabel(connection: "connecting" | "live" | "disconnected") {
  return {
    connecting: "łączenie",
    live: "połączono",
    disconnected: "rozłączono",
  }[connection]
}
