export type WhitePaperPublication={version:string;publishedAt:string;status:"current"|"superseded";summary:string}

// Publications are append-only. A material update adds a new entry and keeps
// the previous one so the public page and source history remain reproducible.
export const publications:WhitePaperPublication[]=[
  {version:"1.2.0-demo",publishedAt:"2026-08-27T12:00:00Z",status:"current",summary:"Rozróżnia parytet wykupu u emitenta od detalicznego kursu transakcyjnego oferowanego przez CASP."},
  {version:"1.1.0-demo",publishedAt:"2026-08-27T00:00:00Z",status:"superseded",summary:"Wyjaśnia, że bieżące pokrycie rezerw jest dodatkową informacją demonstracyjną, a nie wymaganym polem dokumentu informacyjnego EMT."},
  {version:"1.0.0-demo",publishedAt:"2026-08-26T00:00:00Z",status:"superseded",summary:"Pierwszy opis demonstracji badawczej rUSD."},
]

export const currentPublication=publications.find(item=>item.status==="current")!
