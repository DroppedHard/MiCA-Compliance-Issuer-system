import { useQuery } from "@tanstack/react-query"
import { Area, AreaChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts"
import { tokenHistoryQueryKey } from "@/application/queries/token-query"
import { toChartPoint, type TokenObservation } from "@/domain/token"

export function SupplyChart() {
  const { data = [] } = useQuery<TokenObservation[]>({
    queryKey: tokenHistoryQueryKey,
    queryFn: async () => [],
    staleTime: Infinity,
  })
  const points = data.map(toChartPoint)

  return (
    <div className="h-64 w-full" aria-label="Podaż tokenu w ostatnich obserwacjach">
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
            </defs>
            <CartesianGrid stroke="#1e293b" vertical={false} />
            <XAxis dataKey="time" stroke="#64748b" tickLine={false} axisLine={false} minTickGap={32} />
            <YAxis stroke="#64748b" tickLine={false} axisLine={false} width={54} />
            <Tooltip contentStyle={{ background: "#0f172a", border: "1px solid #334155", borderRadius: 12 }} />
            <Area type="monotone" dataKey="supply" name="Podaż" stroke="#2dd4bf" strokeWidth={2} fill="url(#supply)" isAnimationActive={false} />
          </AreaChart>
        </ResponsiveContainer>
      )}
    </div>
  )
}
