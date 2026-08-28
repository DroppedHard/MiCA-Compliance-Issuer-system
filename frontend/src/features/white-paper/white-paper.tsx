import { useQuery } from "@tanstack/react-query"
import { esgQueryOptions } from "@/application/queries/esg-query"
import { reserveQueryOptions } from "@/application/queries/reserve-query"
import { tokenQueryOptions } from "@/application/queries/token-query"
import { formatTokenAmount } from "@/domain/token"
import { currentPublication, publications } from "./content"
import "./white-paper.css"

export function WhitePaper(){
  const token=useQuery(tokenQueryOptions)
  const reserve=useQuery(reserveQueryOptions)
  const esg=useQuery(esgQueryOptions)
  const snapshot=token.data?.snapshot
  return <main className="white-paper">
    <header className="wp-header"><div><p>rUSD RESEARCH DEMO</p><h1>Dokument informacyjny kryptoaktywa</h1><span>Wersja {currentPublication.version} · opublikowano {new Date(currentPublication.publishedAt).toLocaleDateString("pl-PL")}</span></div><button onClick={()=>window.print()}>Zapisz / drukuj PDF</button></header>
    <aside className="wp-warning"><strong>Demonstracja akademicka — nie oferta publiczna.</strong> Ten system nie jest autoryzowanym emitentem EMT, CASP-em ani dowodem zgodności z MiCA. rUSD działa wyłącznie w lokalnej sieci Hardhat i nie przedstawia roszczenia wobec prawdziwego banku.</aside>
    <WpSection title="1. Emitent i cel"><p>Emitent rUSD jest symulowaną instytucją utworzoną na potrzeby eksperymentu magisterskiego. Celem jest pokazanie technicznego przepływu emisji, wykupu, rezerw, monitoringu oraz rozdzielenia odpowiedzialności emitenta i CASP.</p></WpSection>
    <WpSection title="2. Charakterystyka rUSD"><p>rUSD jest demonstracyjnym tokenem inspirowanym EMT, odnoszącym się do jednej waluty urzędowej — dolara amerykańskiego. Emitent stosuje parytet emisji i wykupu 1 rUSD = 1 USD. Jest to odniesienie tokenu do USD, a nie detaliczny kurs transakcyjny oferowany przez CASP. Osobne uproszczenie USD/EUR 1:1 służy wyłącznie demonstracji progów raportowych.</p><Facts items={[["Nazwa",snapshot?.name??"—"],["Symbol",snapshot?.symbol??"rUSD"],["Podaż",snapshot?`${formatTokenAmount(snapshot.totalSupplyRaw,snapshot.decimals)} rUSD`:"—"]]}/></WpSection>
    <WpSection title="3. Prawa posiadacza i wykup"><p>Model zakłada prawo posiadacza do wykupu rUSD po wartości nominalnej 1:1. W demonstracji emitent nie pobiera opłaty za wykup. Klient CASP posiada prawo do tokenów zapisane w ledgerze CASP; przeniesienie do prywatnego portfela pozostaje rozszerzeniem.</p></WpSection>
    <WpSection title="4. Emisja i rezerwy"><p>Mint następuje dopiero po skorelowanym potwierdzeniu wpłaty USD w mockBanku. Wykup spala tokeny i uruchamia wypłatę USD. mockBank upraszcza wycenę wszystkich aktywów rezerwowych do jednej wartości USD. Bieżący procent jest dodatkową informacją demonstracyjną, a nie polem wymaganym dla white paper EMT.</p><Facts items={[["Pokrycie demonstracyjne",reserve.data?.ratioPercent==null?"Brak podaży":`${reserve.data.ratioPercent.toFixed(2)}%`],["Parytet emisji i wykupu","1 rUSD = 1 USD"]]}/></WpSection>
    <WpSection title="5. Technologia"><p>Kontrakt ERC-20 działa lokalnie na Ethereum Hardhat. Role kontraktu kontrolują mint, burn, globalną pauzę i zamrożenie adresów. Backend emitenta jest napisany w Rust, dane trwałe zapisuje w SQLite, a obserwacje udostępnia przez HTTP i SSE.</p><Facts items={[["Chain ID",snapshot?.chainId?.toString()??"—"],["Kontrakt",snapshot?.contractAddress??"—"],["Blok",snapshot?.blockNumber?.toString()??"—"],["Decimals",snapshot?.decimals?.toString()??"—"]]}/></WpSection>
    <WpSection title="6. Ryzyka"><ul><li>ryzyko braku lub niedowartościowania rezerw;</li><li>ryzyko błędów kontraktu, backendu, wyroczni i kluczy administracyjnych;</li><li>ryzyko niedostępności Ethereum, mockBanku lub CASP;</li><li>centralizacja kontroli emitenta i brak rzeczywistej infrastruktury bankowej/HSM;</li><li>estymacyjny, a nie pomiarowy charakter danych środowiskowych.</li></ul></WpSection>
    <WpSection title="7. Metodologia środowiskowa"><p>{esg.data?.methodology.note??"Metodologia jest chwilowo niedostępna."}</p><Facts items={[["Wersja",esg.data?.methodology.version??"—"],["Źródło",esg.data?.methodology.sourceName??"—"],["Najlepsza estymata",esg.data?`${esg.data.methodology.bestGuessEnergyWhPerTransaction} Wh/transakcję`:"—"],["Zakres",esg.data?`${esg.data.methodology.lowerEnergyWhPerTransaction}–${esg.data.methodology.upperEnergyWhPerTransaction} Wh/transakcję`:"—"]]}/>{esg.data&&<a href={esg.data.methodology.sourceUrl} target="_blank" rel="noreferrer">Otwórz źródło metodologii</a>}</WpSection>
    <WpSection title="8. Historia publikacji"><div className="wp-history">{publications.map(item=><article key={item.version}><strong>{item.version}</strong><span>{new Date(item.publishedAt).toLocaleString("pl-PL")}</span><span>{item.status}</span><p>{item.summary}</p></article>)}</div></WpSection>
  </main>
}

function WpSection({title,children}:{title:string;children:React.ReactNode}){return <section><h2>{title}</h2>{children}</section>}
function Facts({items}:{items:[string,string][]}){return <dl>{items.map(([label,value])=><div key={label}><dt>{label}</dt><dd>{value}</dd></div>)}</dl>}
