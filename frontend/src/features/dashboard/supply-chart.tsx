import { useQuery } from "@tanstack/react-query"
import { Area, AreaChart, CartesianGrid, Legend, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts"
import { reserveHistoryQueryKey } from "@/application/queries/reserve-query"
import { tokenHistoryQueryKey } from "@/application/queries/token-query"
import { toChartPoint, type TokenObservation } from "@/domain/token"
import { toReserveValue, type ReserveCoverage, type SupplyReservePoint } from "@/domain/reserve"

type SupplyChartProps = {
  latestToken: TokenObservation
  latestReserve?: ReserveCoverage
}

export function SupplyChart({ latestToken, latestReserve }: SupplyChartProps) {
  const { data = [] } = useQuery<TokenObservation[]>({
    queryKey: tokenHistoryQueryKey,
    queryFn: async () => [],
    staleTime: Infinity,
  })
  const { data: reserveHistory = [] } = useQuery<ReserveCoverage[]>({
    queryKey: reserveHistoryQueryKey,
    queryFn: async () => [],
    staleTime: Infinity,
  })
  const points = buildSupplyReservePoints(data, reserveHistory, latestToken, latestReserve)

  return (
    <div className="h-64 w-full" aria-label="Podaż tokenu i rezerwa bankowa w ostatnich obserwacjach">
      {points.length < 2 ? (
        <div className="flex h-full items-center justify-center rounded-xl border border-dashed border-slate-700 text-sm text-slate-500">
          Oczekiwanie na kolejną obserwację…
        </div>
      ) : (
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={points} margin={{ left: 0, right: 12, top: 8, bottom: 0 }} accessibilityLayer>
            <defs>
              <linearGradient id="supply" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#2dd4bf" stopOpacity={0.35} />
                <stop offset="95%" stopColor="#2dd4bf" stopOpacity={0} />
              </linearGradient>
              <linearGradient id="reserve" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#a78bfa" stopOpacity={0.2} />
                <stop offset="95%" stopColor="#a78bfa" stopOpacity={0} />
              </linearGradient>
            </defs>
            <CartesianGrid stroke="#1e293b" vertical={false} />
            <XAxis dataKey="time" stroke="#64748b" tickLine={false} axisLine={false} minTickGap={32} />
            <YAxis stroke="#64748b" tickLine={false} axisLine={false} width={54} />
            <Tooltip contentStyle={{ background: "#0f172a", border: "1px solid #334155", borderRadius: 12 }} />
            <Legend verticalAlign="top" align="right" iconType="line" wrapperStyle={{ paddingBottom: 8 }} />
            <Area type="monotone" dataKey="supply" name="Podaż tokenu (rUSD)" stroke="#2dd4bf" strokeWidth={2} fill="url(#supply)" connectNulls isAnimationActive={false} />
            <Area type="monotone" dataKey="reserve" name="Rezerwa bankowa (USD)" stroke="#a78bfa" strokeWidth={2} fill="url(#reserve)" connectNulls isAnimationActive={false} />
          </AreaChart>
        </ResponsiveContainer>
      )}
    </div>
  )
}

export function buildSupplyReservePoints(
  tokenHistory: TokenObservation[],
  reserveHistory: ReserveCoverage[],
  latestToken: TokenObservation,
  latestReserve?: ReserveCoverage,
): SupplyReservePoint[] {
  const rawEvents = [
    ...tokenHistory.map((value) => ({ observedAtUnixMs: value.observedAtUnixMs, supply: toChartPoint(value).supply })),
    ...reserveHistory.map((value) => ({ observedAtUnixMs: value.observedAtUnixMs, reserve: toReserveValue(value) })),
    { observedAtUnixMs: latestToken.observedAtUnixMs, supply: toChartPoint(latestToken).supply },
    ...(latestReserve ? [{ observedAtUnixMs: latestReserve.observedAtUnixMs, reserve: toReserveValue(latestReserve) }] : []),
  ].sort((left, right) => left.observedAtUnixMs - right.observedAtUnixMs)
  const events = [...rawEvents.reduce((grouped, event) => {
    const previous = grouped.get(event.observedAtUnixMs)
    grouped.set(event.observedAtUnixMs, { ...previous, ...event })
    return grouped
  }, new Map<number, { observedAtUnixMs: number; supply?: number; reserve?: number }>()).values()]

  let supply: number | undefined
  let reserve: number | undefined
  return events.map((event) => {
    supply = event.supply ?? supply
    reserve = event.reserve ?? reserve
    return {
      observedAtUnixMs: event.observedAtUnixMs,
      time: new Date(event.observedAtUnixMs).toLocaleTimeString("pl-PL", { hour: "2-digit", minute: "2-digit", second: "2-digit" }),
      supply,
      reserve,
    }
  })
}
