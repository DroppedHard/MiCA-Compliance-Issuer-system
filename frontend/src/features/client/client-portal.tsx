import { useMemo, useState } from "react"
import { ArrowDownToLine, ArrowLeftRight, ArrowRight, Building2, Check, ChevronRight, CircleDollarSign, ExternalLink, Info, Landmark, LockKeyhole, ShieldCheck, WalletCards } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"

type OperationMode = "purchase" | "redemption"

const WALLET = "0x7099…79C8"
const MOCK_BALANCE = 1250

export function ClientPortal() {
  const [connected, setConnected] = useState(false)
  const [mode, setMode] = useState<OperationMode>("purchase")
  const [amount, setAmount] = useState("100")
  const numericAmount = parsePositiveAmount(amount)
  const feePercent = mode === "redemption" ? 0 : 0
  const fee = numericAmount * feePercent / 100
  const net = Math.max(0, numericAmount - fee)
  const canContinue = connected && numericAmount > 0 && (mode === "purchase" || numericAmount <= MOCK_BALANCE)
  const steps = useMemo(() => mode === "purchase" ? purchaseSteps : redemptionSteps, [mode])

  return (
    <main className="mx-auto min-h-screen max-w-7xl p-5 md:p-8">
      <header className="mb-8 flex flex-col gap-5 md:flex-row md:items-center md:justify-between">
        <div>
          <div className="mb-3 flex items-center gap-2">
            <div className="grid size-9 place-items-center rounded-xl bg-teal-400 text-slate-950"><CircleDollarSign className="size-5" /></div>
            <span className="font-mono text-xs uppercase tracking-[0.24em] text-teal-300">Research Euro EMT</span>
          </div>
          <h1 className="text-3xl font-semibold tracking-tight text-white md:text-4xl">Twój portfel rUSD</h1>
          <p className="mt-2 max-w-2xl text-slate-400">Makieta zakupu i wykupu tokenu z perspektywy klienta.</p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <a href="/" className="inline-flex items-center gap-2 rounded-lg border border-slate-700 px-3 py-2 text-xs font-medium text-slate-300 transition hover:bg-slate-800">Panel administratora <ExternalLink className="size-3.5" /></a>
          <button type="button" onClick={() => setConnected(value => !value)} className={`inline-flex items-center gap-2 rounded-lg px-4 py-2.5 text-sm font-semibold transition ${connected ? "border border-emerald-400/30 bg-emerald-400/10 text-emerald-200" : "bg-teal-400 text-slate-950 hover:bg-teal-300"}`}>
            {connected ? <><Check className="size-4" /> {WALLET}</> : <><WalletCards className="size-4" /> Połącz MetaMask</>}
          </button>
        </div>
      </header>

      <div className="mb-5 flex items-start gap-3 rounded-xl border border-blue-400/20 bg-blue-400/5 p-4 text-sm text-blue-100">
        <Info className="mt-0.5 size-5 shrink-0 text-blue-300" />
        <div><p className="font-medium">Widok demonstracyjny</p><p className="mt-1 text-xs leading-5 text-blue-200/70">Przyciski prezentują planowany proces. Nie uruchamiają jeszcze MetaMaska, płatności ani transakcji blockchainowych.</p></div>
      </div>

      <section className="grid gap-4 md:grid-cols-3">
        <SummaryCard icon={WalletCards} label="Dostępne saldo" value={`${formatAmount(MOCK_BALANCE)} rUSD`} detail={connected ? WALLET : "Połącz portfel, aby potwierdzić adres"} />
        <SummaryCard icon={ShieldCheck} label="Status tokenu" value="Aktywny" detail="Zakup i wykup dostępne" accent="emerald" />
        <SummaryCard icon={Landmark} label="Kurs wykupu" value="1 rUSD = 1 USD" detail="Tryb normalny · bez opłaty" />
      </section>

      <section className="mt-4 grid gap-4 lg:grid-cols-[1.15fr_0.85fr]">
        <Card>
          <CardHeader>
            <CardTitle>Nowa operacja</CardTitle>
            <CardDescription>Wybierz, czy chcesz kupić nowe tokeny, czy wymienić posiadane rUSD na USD.</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="mb-6 grid grid-cols-2 rounded-xl bg-slate-950/70 p-1">
              <ModeButton active={mode === "purchase"} onClick={() => setMode("purchase")} icon={ArrowDownToLine}>Kup rUSD</ModeButton>
              <ModeButton active={mode === "redemption"} onClick={() => setMode("redemption")} icon={ArrowLeftRight}>Wykup rUSD</ModeButton>
            </div>

            <label className="text-sm font-medium text-slate-200" htmlFor="operation-amount">{mode === "purchase" ? "Kwota zakupu" : "Liczba tokenów do wykupu"}</label>
            <div className="mt-2 flex items-center rounded-xl border border-slate-700 bg-slate-950/70 px-4 focus-within:border-teal-400/60">
              <input id="operation-amount" inputMode="decimal" value={amount} onChange={event => setAmount(event.target.value)} className="min-w-0 flex-1 bg-transparent py-4 text-2xl font-semibold text-white outline-none" aria-describedby="amount-help" />
              <span className="font-mono text-sm text-slate-400">{mode === "purchase" ? "USD" : "rUSD"}</span>
            </div>
            <div id="amount-help" className="mt-2 flex justify-between text-xs text-slate-500"><span>Minimalna kwota: 1,00</span>{mode === "redemption" && <button type="button" onClick={() => setAmount(String(MOCK_BALANCE))} className="text-teal-300 hover:text-teal-200">Użyj całego salda</button>}</div>
            {mode === "redemption" && numericAmount > MOCK_BALANCE && <p className="mt-3 text-sm text-rose-300">Kwota przekracza dostępne saldo rUSD.</p>}

            <div className="my-6 space-y-3 rounded-xl border border-slate-800 bg-slate-950/50 p-4 text-sm">
              <QuoteRow label={mode === "purchase" ? "Wpłata" : "Wartość nominalna"} value={`${formatAmount(numericAmount)} USD`} />
              {mode === "redemption" && <QuoteRow label="Opłata płynnościowa" value={feePercent === 0 ? "0,00 USD" : `${formatAmount(fee)} USD`} />}
              <div className="border-t border-slate-800 pt-3"><QuoteRow label={mode === "purchase" ? "Otrzymasz" : "Wypłata netto"} value={`${formatAmount(net)} ${mode === "purchase" ? "rUSD" : "USD"}`} strong /></div>
            </div>

            <button type="button" disabled={!canContinue} className="flex w-full items-center justify-center gap-2 rounded-xl bg-teal-400 px-4 py-3.5 font-semibold text-slate-950 transition hover:bg-teal-300 disabled:cursor-not-allowed disabled:bg-slate-700 disabled:text-slate-400">
              {!connected ? "Najpierw połącz MetaMask" : mode === "purchase" ? "Przejdź do wpłaty" : "Sprawdź i zablokuj tokeny"}<ArrowRight className="size-4" />
            </button>
            <p className="mt-3 text-center text-xs leading-5 text-slate-500">{mode === "purchase" ? "Mint wykona emitent po potwierdzeniu wpłaty. Nie podpisujesz transakcji mint." : "MetaMask poprosi najpierw o zgodę na dokładną kwotę, a następnie o blokadę tokenów w escrow."}</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader><CardTitle>Jak przebiegnie operacja?</CardTitle><CardDescription>{mode === "purchase" ? "Zakup po potwierdzonej wpłacie bankowej." : "Bezpieczny wykup z użyciem kontraktu escrow."}</CardDescription></CardHeader>
          <CardContent className="space-y-1">
            {steps.map((step, index) => <ProcessStep key={step.title} index={index + 1} {...step} last={index === steps.length - 1} />)}
          </CardContent>
        </Card>
      </section>

      <section className="mt-4 grid gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader><CardTitle>Ostatnia operacja</CardTitle><CardDescription>Przykład procesu, który można bezpiecznie wznowić po zamknięciu strony.</CardDescription></CardHeader>
          <CardContent>
            <div className="flex flex-col gap-4 rounded-xl border border-slate-800 bg-slate-950/60 p-4 sm:flex-row sm:items-center sm:justify-between">
              <div><div className="flex items-center gap-2"><p className="font-medium text-white">Zakup 250 rUSD</p><Badge className="border-amber-400/30 bg-amber-400/10 text-amber-200">Oczekuje na mint</Badge></div><p className="mt-2 font-mono text-xs text-slate-500">Operacja 7c0a…91ef</p></div>
              <div className="text-left sm:text-right"><p className="text-sm text-slate-300">Środki potwierdzone</p><p className="mt-1 text-xs text-slate-500">Backend bezpiecznie ponowi mint</p></div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader><CardTitle>Bezpieczeństwo operacji</CardTitle><CardDescription>Co pozostaje pod kontrolą klienta.</CardDescription></CardHeader>
          <CardContent className="grid gap-3 sm:grid-cols-2">
            <SafetyItem icon={LockKeyhole} title="Klucz zostaje w portfelu">Backend nigdy nie otrzymuje klucza prywatnego ani frazy seed.</SafetyItem>
            <SafetyItem icon={Building2} title="Środki są uzgadniane">Każda wpłata, blokada i wypłata ma jeden trwały identyfikator.</SafetyItem>
          </CardContent>
        </Card>
      </section>
    </main>
  )
}

const purchaseSteps = [
  { icon: WalletCards, title: "Potwierdzenie portfela", text: "Podpis wiadomości potwierdza, do kogo trafią rUSD." },
  { icon: Landmark, title: "Wpłata USD", text: "mockBank potwierdza środki z unikalnym numerem operacji." },
  { icon: CircleDollarSign, title: "Mint tokenów", text: "Emitent tworzy rUSD dopiero po zarezerwowaniu wpłaty." },
  { icon: Check, title: "Potwierdzenie", text: "Tokeny pojawiają się po potwierdzeniu transakcji blockchainowej." },
]

const redemptionSteps = [
  { icon: WalletCards, title: "Akceptacja warunków", text: "Widzisz kwotę brutto, opłatę i wypłatę netto." },
  { icon: LockKeyhole, title: "Blokada w escrow", text: "MetaMask zatwierdza dokładną liczbę tokenów do wykupu." },
  { icon: Landmark, title: "Wypłata USD", text: "mockBank wypłaca środki, gdy blokada jest potwierdzona." },
  { icon: Check, title: "Burn tokenów", text: "Tokeny są spalane dopiero po potwierdzonej wypłacie." },
]

function ModeButton({ active, onClick, icon: Icon, children }: { active: boolean; onClick: () => void; icon: typeof ArrowDownToLine; children: string }) {
  return <button type="button" onClick={onClick} className={`flex items-center justify-center gap-2 rounded-lg px-3 py-2.5 text-sm font-medium transition ${active ? "bg-slate-800 text-white shadow" : "text-slate-500 hover:text-slate-300"}`}><Icon className="size-4" />{children}</button>
}

function SummaryCard({ icon: Icon, label, value, detail, accent = "teal" }: { icon: typeof WalletCards; label: string; value: string; detail: string; accent?: "teal" | "emerald" }) {
  return <Card><CardHeader className="flex-row items-center justify-between pb-3"><CardDescription>{label}</CardDescription><Icon className={`size-4 ${accent === "emerald" ? "text-emerald-400" : "text-teal-400"}`} /></CardHeader><CardContent><p className="text-2xl font-semibold text-white">{value}</p><p className="mt-1 text-xs text-slate-500">{detail}</p></CardContent></Card>
}

function QuoteRow({ label, value, strong = false }: { label: string; value: string; strong?: boolean }) {
  return <div className="flex items-center justify-between gap-4"><span className="text-slate-400">{label}</span><span className={strong ? "text-base font-semibold text-white" : "text-slate-200"}>{value}</span></div>
}

function ProcessStep({ index, icon: Icon, title, text, last }: { index: number; icon: typeof WalletCards; title: string; text: string; last: boolean }) {
  return <div className="grid grid-cols-[2.5rem_1fr] gap-3"><div className="flex flex-col items-center"><div className="grid size-9 place-items-center rounded-full border border-teal-400/30 bg-teal-400/10 text-teal-300"><Icon className="size-4" /></div>{!last && <div className="my-1 min-h-8 w-px flex-1 bg-slate-800" />}</div><div className="pb-5"><div className="flex items-center gap-2"><span className="text-xs text-slate-600">{index}</span><p className="font-medium text-slate-200">{title}</p><ChevronRight className="size-3 text-slate-700" /></div><p className="mt-1 text-xs leading-5 text-slate-500">{text}</p></div></div>
}

function SafetyItem({ icon: Icon, title, children }: { icon: typeof LockKeyhole; title: string; children: string }) {
  return <div className="rounded-xl border border-slate-800 bg-slate-950/60 p-4"><Icon className="mb-3 size-5 text-teal-400" /><p className="text-sm font-medium text-slate-200">{title}</p><p className="mt-1 text-xs leading-5 text-slate-500">{children}</p></div>
}

export const parsePositiveAmount = (value: string): number => {
  const parsed = Number(value.replace(",", "."))
  return Number.isFinite(parsed) && parsed > 0 ? parsed : 0
}

const formatAmount = (value: number): string => value.toLocaleString("pl-PL", { minimumFractionDigits: 2, maximumFractionDigits: 2 })
