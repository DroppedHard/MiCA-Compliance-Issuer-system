# Frontend emitenta

Aplikacja React udostępnia panel administratora oraz publiczny dokument informacyjny. Pierwszy stan pobiera przez HTTP, a aktualizacje podaży, rezerw, ESG i stanu tokenu przez SSE. Reguły domenowe pozostają w backendzie.

```powershell
pnpm install --frozen-lockfile
pnpm dev
pnpm test
pnpm build
```

Panel działa pod `http://127.0.0.1:5173/`, a dokument pod `/white-paper`.

