import { useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { Activity, Blocks, CircleDollarSign, CircleHelp, Database, Leaf, Radio, ShieldCheck, TriangleAlert, Zap } from "lucide-react"
import { reserveQueryOptions } from "@/application/queries/reserve-query"
import { assetStateQueryOptions } from "@/application/queries/asset-state-query"
import { useAssetStateStream } from "@/application/realtime/use-asset-state-stream"
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
import { assetStateLabel } from "@/domain/asset-state"
import { SupplyChart, type SupplyChartPreset } from "./supply-chart"
import { EsgEnergyChart } from "./esg-energy-chart"
import { WindDownControl } from "./wind-down-control"
import { ReserveAdjustmentControl } from "./reserve-adjustment-control"

const chartPresets: Array<SupplyChartPreset & { pollLabel: string; rangeLabel: string }> = [
  { id: "live", sampleIntervalMs: 10_000, rangeMs: 60 * 60_000, pollLabel: "co 10 s", rangeLabel: "zakres 1 h" },
  { id: "medium", sampleIntervalMs: 60_000, rangeMs: 4 * 60 * 60_000, pollLabel: "co 1 min", rangeLabel: "zakres 4 h" },
  { id: "overview", sampleIntervalMs: 60 * 60_000, rangeMs: 12 * 60 * 60_000, pollLabel: "co 1 godz.", rangeLabel: "zakres 12 h" },
]

export function AdminDashboard() {
  const token = useQuery(tokenQueryOptions)
  const esg = useQuery(esgQueryOptions)
  const esgHistory = useQuery(esgHistoryQueryOptions)
  const reserve = useQuery(reserveQueryOptions)
  const assetState = useQuery(assetStateQueryOptions)
  const [detailsOpen, setDetailsOpen] = useState(false)
  const [chartPreset, setChartPreset] = useState<SupplyChartPreset>(chartPresets[0])
  const connection = useTokenStream()
  useEsgStream()
  useReserveStream()
  useAssetStateStream()

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
    <main className="issuer-dashboard">
      <header className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="mb-2 font-mono text-xs uppercase tracking-[0.28em] text-teal-400">Środowisko badawcze</p>
          <h1 className="text-2xl font-semibold tracking-tight text-white md:text-3xl">Panel zarządzania EMT</h1>
          <p className="mt-2 max-w-2xl text-slate-400">Bieżący podgląd lokalnie wdrożonego kryptoaktywa oraz demonstracyjne sterowanie jego cyklem życia.</p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <Badge className={connection === "live" ? "border-teal-400/30 bg-teal-400/10 text-teal-300" : "border-amber-400/30 bg-amber-400/10 text-amber-300"}>
            <Radio className="mr-1.5 size-3" /> SSE {connectionLabel(connection)}
          </Badge>
        </div>
      </header>

      <section className="dashboard-overview">
        <div className="parity-summary" title="Stałe odniesienie emisji i wykupu"><span>Parytet emitenta</span><strong>1 rUSD = 1 USD</strong></div>
        <Metric icon={CircleDollarSign} label="Całkowita podaż" value={`${supply} ${snapshot.symbol}`} detail={`${snapshot.decimals} miejsc dziesiętnych`} />
        <Metric icon={Blocks} label="Najnowszy blok" value={snapshot.blockNumber.toLocaleString("pl-PL")} detail={`Identyfikator sieci ${snapshot.chainId}`} />
        <Metric icon={Database} label="Kontrakt" value={shortenAddress(snapshot.contractAddress)} detail={snapshot.name} mono />
        <Metric icon={Activity} label="Czas obserwacji" value={new Date(observedAtUnixMs).toLocaleTimeString("pl-PL")} detail={new Date(observedAtUnixMs).toLocaleDateString("pl-PL")} />
      </section>

      <section className="dashboard-workspace">
        <aside className="admin-actions">
          <Card className={reserve.data?.status === "undercollateralized" ? "border-rose-500/40" : undefined}>
            <CardHeader><CardTitle>Rezerwy</CardTitle><CardDescription>Pokrycie podaży i testowa korekta mockBanku.</CardDescription></CardHeader>
            <CardContent>
              {reserve.isPending && <Skeleton className="h-24" />}
              {reserve.isError && <div className="flex items-center gap-2 rounded-lg border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-200"><TriangleAlert className="size-4" /> Brak danych z mockBanku.</div>}
              {reserve.data && <div className="reserve-summary">
                <StatusRow label="Rezerwa" value={`${reserve.data.reserveBalanceUsd} USD`} />
                <StatusRow label="Zobowiązanie" value={`${reserve.data.liabilityUsd} USD`} />
                <StatusRow label="Pokrycie" value={reserve.data.ratioPercent === null ? "Brak podaży" : `${reserve.data.ratioPercent.toLocaleString("pl-PL", { maximumFractionDigits: 2 })}%`} />
                <StatusRow label={Number(reserve.data.surplusUsd) >= 0 ? "Nadwyżka" : "Niedobór"} value={`${reserve.data.surplusUsd} USD`} />
              </div>}
              {reserve.data && <p className={`reserve-state ${reserve.data.status === "covered" ? "covered" : "uncovered"}`}>{reserve.data.status === "covered" ? "Rezerwa pokrywa podaż rUSD." : "Nowa emisja jest zablokowana; wykup pozostaje dostępny."}</p>}
              <ReserveAdjustmentControl />
            </CardContent>
          </Card>
          <Card className="border-rose-500/20">
            <CardHeader><CardTitle>Wygaszanie tokenu</CardTitle><CardDescription>Nieodwracalna blokada emisji i zwykłych transferów.</CardDescription></CardHeader>
            <CardContent><WindDownControl state={assetState.data?.state} /></CardContent>
          </Card>
        </aside>

        <Card className="chart-panel">
          <CardHeader className="flex-row items-start justify-between gap-4">
            <div><CardTitle>Podaż i rezerwa</CardTitle><CardDescription>Dane z cache backendu otrzymywane przez SSE.</CardDescription></div>
            <div className="chart-presets" aria-label="Ustawienia zakresu wykresu">
              {chartPresets.map((preset) => <button key={preset.id} className={chartPreset.id === preset.id ? "active" : ""} onClick={() => setChartPreset(preset)}><strong>{preset.pollLabel}</strong><span>{preset.rangeLabel}</span></button>)}
            </div>
          </CardHeader>
          <CardContent><SupplyChart latestToken={token.data} latestReserve={reserve.data} preset={chartPreset} /></CardContent>
        </Card>

        <Card className="system-panel">
          <CardHeader><CardTitle>Stan systemu</CardTitle><CardDescription>Bieżący stan połączeń między warstwami.</CardDescription></CardHeader>
          <CardContent className="space-y-4">
            <StatusRow label="Ostatni odczyt backendu" value={new Date(observedAtUnixMs).toLocaleTimeString("pl-PL")} />
            <StatusRow label="Cache backendu" value="Pamięć procesu · 24 godz." />
            <StatusRow label="Transport" value="SSE + inicjalizacja HTTP" />
            {assetState.isPending && <Skeleton className="h-28" />}
            {assetState.isError && <div className="rounded-xl border border-amber-500/30 bg-amber-500/10 p-4 text-xs text-amber-200">Nie udało się odczytać decyzji emitenta.</div>}
            {assetState.data && <div className={`rounded-xl border p-4 text-xs leading-6 ${assetStateClasses(assetState.data.state)}`}>
              <ShieldCheck className="mb-2 size-5" />
              <div className="flex items-center justify-between gap-3"><strong className="text-sm">{assetStateLabel(assetState.data.state)}</strong><span className="font-mono text-[10px]">{assetState.data.policyVersion}</span></div>
              <p className="mt-2">{assetState.data.reason}</p>
              <p className="mt-2 opacity-70">Ostatnia decyzja: {new Date(assetState.data.updatedAtUnixMs).toLocaleString("pl-PL")}</p>
            </div>}
          </CardContent>
        </Card>
        {esg.data && <Card className="esg-panel">
          <CardHeader className="flex-row items-start justify-between gap-3">
            <div><CardTitle>Estymowane zużycie energii tokena</CardTitle><CardDescription>Aktywność rUSD · {esg.data.currentDay.dateUtc} · dane prowizoryczne</CardDescription></div>
            <button aria-label="Pokaż sposób obliczania estymaty" title="Jak obliczono estymatę?" className="flex size-8 shrink-0 items-center justify-center rounded-full border border-slate-700 text-slate-300 hover:border-teal-500/60 hover:bg-slate-800 hover:text-teal-300" onClick={() => setDetailsOpen(true)}><CircleHelp className="size-4" /></button>
          </CardHeader>
          <CardContent>
            <div className="esg-metrics">
              <EsgMetric icon={Activity} label="Transakcje" value={esg.data.currentDay.transactionCount.toLocaleString("pl-PL")} />
              <EsgMetric icon={Zap} label="Energia" value={`${esg.data.currentDay.energyBestGuessWh.toLocaleString("pl-PL", { maximumFractionDigits: 3 })} Wh`} />
              <EsgMetric icon={Leaf} label="Emisje" value={`${esg.data.currentDay.emissionsGCo2e.toLocaleString("pl-PL", { maximumFractionDigits: 3 })} g CO₂e`} />
              <EsgMetric icon={Blocks} label="Blok" value={esg.data.lastProcessedBlock.toLocaleString("pl-PL")} />
            </div>
            <EnergyMixBar renewablePercent={esg.data.methodology.renewablePercent} nuclearPercent={esg.data.methodology.nuclearPercent} fossilPercent={esg.data.methodology.fossilPercent} renewableWh={esg.data.currentDay.renewableEnergyWh} nuclearWh={esg.data.currentDay.nuclearEnergyWh} fossilWh={esg.data.currentDay.fossilEnergyWh} />
            {esgHistory.data && <div className="esg-history">
              <div className="mb-2 flex items-center justify-between"><div><p className="text-sm font-medium text-slate-200">Estymowane zużycie energii · 7 dni</p><p className="text-[10px] text-slate-500">Najlepsza estymata wraz z dolnym i górnym scenariuszem</p></div><Badge className="border-slate-700 bg-slate-800 text-slate-300">Wh</Badge></div>
              <EsgEnergyChart days={esgHistory.data.days} />
            </div>}
          </CardContent>
        </Card>}
      </section>
      {esg.data && <Dialog open={detailsOpen} onClose={() => setDetailsOpen(false)} title="O estymacie zużycia energii">
          <div className="space-y-4 text-sm leading-6 text-slate-300">
            <p>Panel przedstawia <strong>estymowane zużycie energii przypisane transakcjom rUSD</strong>. Nie jest to bezpośredni pomiar energii zużytej przez token ani dodatkowego obciążenia wywołanego przez pojedynczą transakcję.</p>
            <div className="rounded-lg border border-slate-800 bg-slate-950/50 p-3">
              <p className="font-medium text-slate-100">Podstawa estymaty</p>
              <p className="mt-1">Dane Cambridge opisują roczne zużycie energii całej sieci Ethereum PoS w trzech scenariuszach: <strong>1,26 GWh</strong> (dolny), <strong>7,87 GWh</strong> (najlepsza estymata) oraz <strong>11,49 GWh</strong> (górny).</p>
            </div>
            <div>
              <p className="font-medium text-slate-100">Sposób przypisania energii do rUSD</p>
              <p className="mt-1">Każdy scenariusz jest dzielony przez założone <strong>{esg.data.methodology.annualTransactionsAssumption.toLocaleString("pl-PL")}</strong> transakcji Ethereum rocznie. Następnie liczba zaobserwowanych transakcji rUSD jest mnożona przez uzyskane współczynniki: <strong>{esg.data.methodology.lowerEnergyWhPerTransaction}–{esg.data.methodology.upperEnergyWhPerTransaction} Wh na transakcję</strong>, przy najlepszej estymacie <strong>{esg.data.methodology.bestGuessEnergyWhPerTransaction} Wh</strong>.</p>
            </div>
            <p>Dolna i górna wartość pokazują alternatywne scenariusze sprzętu i hostingu. Nie są statystycznym przedziałem ufności.</p>
            <p>Prezentowany miks energetyczny wynosi: {esg.data.methodology.renewablePercent}% OZE, {esg.data.methodology.nuclearPercent}% energii jądrowej i {esg.data.methodology.fossilPercent}% paliw kopalnych.</p>
            <a className="inline-flex text-teal-300 underline decoration-teal-500/40 underline-offset-4" href={esg.data.methodology.sourceUrl} target="_blank" rel="noreferrer">Źródło danych: {esg.data.methodology.sourceName}</a>
            <p className="text-xs text-slate-500">Metodologia demonstracyjna: {esg.data.methodology.version}</p>
          </div>
      </Dialog>}
    </main>
  )
}

function EsgMetric({ icon: Icon, label, value }: { icon: typeof Activity; label: string; value: string }) { return <div className="rounded-lg border border-slate-800 bg-slate-950/60 p-2.5"><div className="flex items-center gap-2"><Icon className="size-3.5 text-teal-400" /><p className="text-[10px] text-slate-500">{label}</p></div><p className="mt-1.5 truncate text-base font-semibold text-white">{value}</p></div> }
function EnergyMixBar({ renewablePercent, nuclearPercent, fossilPercent, renewableWh, nuclearWh, fossilWh }: { renewablePercent: number; nuclearPercent: number; fossilPercent: number; renewableWh: number; nuclearWh: number; fossilWh: number }) {
  const parts = [
    { label: "Odnawialne źródła energii", short: "OZE", percent: renewablePercent, value: renewableWh, color: "bg-emerald-400", dot: "bg-emerald-400" },
    { label: "Energia jądrowa", short: "Jądrowa", percent: nuclearPercent, value: nuclearWh, color: "bg-violet-400", dot: "bg-violet-400" },
    { label: "Paliwa kopalne", short: "Kopalne", percent: fossilPercent, value: fossilWh, color: "bg-amber-400", dot: "bg-amber-400" },
  ]
  return <div className="mt-3 rounded-lg border border-slate-800 bg-slate-950/40 p-2.5"><div className="mb-2 flex items-center justify-between gap-3"><p className="text-xs font-medium text-slate-200">Miks źródeł energii</p><p className="text-sm font-semibold text-white">{(renewableWh + nuclearWh + fossilWh).toLocaleString("pl-PL", { maximumFractionDigits: 3 })} Wh</p></div><div className="flex h-3 overflow-hidden rounded-full bg-slate-800" aria-label="Udział źródeł energii">{parts.map(part=><div key={part.short} className={`${part.color} h-full`} style={{ width: `${part.percent}%` }} title={`${part.label}: ${part.percent}%`} />)}</div><div className="mt-2 flex gap-3">{parts.map(part=><div key={part.label} className="flex min-w-0 flex-1 items-start gap-1.5"><span className={`mt-1 size-2 shrink-0 rounded-full ${part.dot}`} /><div className="min-w-0"><p className="truncate text-[10px] text-slate-400">{part.short}</p><p className="truncate text-[11px] font-medium text-slate-100">{part.percent}% · {part.value.toLocaleString("pl-PL", { maximumFractionDigits: 2 })} Wh</p></div></div>)}</div></div>
}

function Metric({ icon: Icon, label, value, detail, mono = false }: { icon: typeof Activity; label: string; value: string; detail: string; mono?: boolean }) {
  return <article className="overview-metric" title={detail}><Icon className="size-4 text-teal-400" /><span>{label}</span><strong className={mono ? "font-mono" : ""}>{value}</strong></article>
}

function StatusRow({ label, value }: { label: string; value: string }) {
  return <div className="flex items-center justify-between border-b border-slate-800 pb-3 text-sm"><span className="text-slate-400">{label}</span><span className="font-medium text-slate-200">{value}</span></div>
}

function DashboardLoading() {
  return <main className="issuer-dashboard"><Skeleton className="mb-4 h-16 w-96 max-w-full" /><div className="dashboard-metrics">{Array.from({ length: 4 }).map((_, index) => <Skeleton key={index} className="h-24" />)}</div><Skeleton className="mt-3 h-72" /></main>
}

function connectionLabel(connection: "connecting" | "live" | "disconnected") {
  return {
    connecting: "łączenie",
    live: "połączono",
    disconnected: "rozłączono",
  }[connection]
}

function assetStateClasses(state: "active" | "warning" | "mint_blocked" | "data_unavailable" | "wind_down") {
  if (state === "active") return "border-emerald-500/30 bg-emerald-500/10 text-emerald-300"
  if (state === "warning") return "border-amber-500/30 bg-amber-500/10 text-amber-200"
  if (state === "data_unavailable") return "border-slate-700 bg-slate-900 text-slate-300"
  return "border-rose-500/30 bg-rose-500/10 text-rose-300"
}
