import { Area, CartesianGrid, ComposedChart, Line, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts"
import type { EsgEstimate } from "@/domain/esg"

export const toEsgChartPoints = (days: readonly EsgEstimate[]) => days.map((day) => ({
  date: new Date(`${day.dateUtc}T00:00:00Z`).toLocaleDateString("pl-PL", { day: "2-digit", month: "2-digit" }),
  range: [day.energyLowerWh, day.energyUpperWh] as const,
  best: day.energyBestGuessWh,
  origin: day.dataOrigin,
}))

export function EsgEnergyChart({ days }: { days: readonly EsgEstimate[] }) {
  const data = toEsgChartPoints(days)
  return <div className="h-72 w-full" aria-label="Estymowane zużycie energii w ostatnich siedmiu dniach">
    <ResponsiveContainer width="100%" height="100%">
      <ComposedChart data={data} margin={{ top: 10, right: 14, bottom: 0, left: 4 }} accessibilityLayer>
        <CartesianGrid stroke="#1e293b" vertical={false} />
        <XAxis dataKey="date" stroke="#64748b" tickLine={false} axisLine={false} />
        <YAxis stroke="#64748b" tickLine={false} axisLine={false} width={72} unit=" Wh" />
        <Tooltip formatter={(value, name) => name === "Zakres Cambridge" ? [`${(value as number[]).map((number) => number.toLocaleString("pl-PL", { maximumFractionDigits: 2 })).join(" – ")} Wh`, name] : [`${Number(value).toLocaleString("pl-PL", { maximumFractionDigits: 2 })} Wh`, name]} contentStyle={{ background: "#0f172a", border: "1px solid #334155", borderRadius: 12 }} />
        <Area type="monotone" dataKey="range" name="Zakres Cambridge" stroke="none" fill="#2dd4bf" fillOpacity={0.16} isAnimationActive={false} />
        <Line type="monotone" dataKey="best" name="Najlepsza estymata" stroke="#2dd4bf" strokeWidth={2.5} dot={{ fill: "#2dd4bf", r: 3 }} isAnimationActive={false} />
      </ComposedChart>
    </ResponsiveContainer>
  </div>
}
