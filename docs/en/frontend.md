# Issuer frontend

The React application provides the issuer administration panel and public information document. It bootstraps over HTTP and receives supply, reserve, ESG, and lifecycle updates over SSE. Domain decisions remain in the backend.

```powershell
pnpm install --frozen-lockfile
pnpm dev
pnpm test
pnpm build
```

The dashboard is served at `http://127.0.0.1:5173/` and the information document at `/white-paper`.
