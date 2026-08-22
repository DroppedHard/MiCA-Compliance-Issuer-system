# Crypto-asset admin frontend

## ESG view

The first ESG value is fetched from `GET /api/v1/esg` and seven-day history from `GET /api/v1/esg/daily` through React Query and Effect. Later current-day values arrive as `esg` events from `/api/v1/esg/stream`. The Polish chart draws the best-guess line inside Cambridge's lower/upper scenario band. “Jak obliczono?” explains the scenario assumptions and the demo's 400-million-transactions allocation.

React admin dashboard for the local `ResearchEuroEMT` experiment. The UI bootstraps the latest observation with HTTP and receives subsequent observations through Server-Sent Events (SSE).

## Data flow

```text
Rust poller → 24-hour backend cache → GET /api/v1/token
                                   └→ GET /api/v1/token/stream (SSE)
                                                    ↓
Effect runtime decoding → TanStack Query cache → React dashboard
```

TanStack Query is the only owner of remote state in React. The initial HTTP query fills its cache; SSE pushes use `setQueryData` to update the same entry. Effect validates unknown JSON at the network boundary and provides `pipe` for pure presentation transforms. The browser's native `EventSource` handles SSE reconnection.

## Layers

- `domain/`: runtime schemas, types, and pure transformations;
- `infrastructure/api/`: HTTP adapter;
- `infrastructure/realtime/`: SSE adapter;
- `application/queries/`: query keys and TanStack Query configuration;
- `application/realtime/`: React integration that updates query cache;
- `features/dashboard/`: dashboard composition and chart;
- `components/ui/`: locally owned shadcn/ui components.

Recharts was selected instead of Chart.js because shadcn/ui's official Chart component uses Recharts. This keeps future shadcn chart adoption on the same underlying library.

## Local setup tutorial

### Prerequisites

- Node.js supported by Vite;
- pnpm (`corepack enable` can make it available with recent Node.js installations);
- the Rust backend running at `http://127.0.0.1:3000`.

Install dependencies once from the repository root:

```powershell
cd frontend
pnpm install
```

Verify the project:

```powershell
pnpm test
pnpm build
```

### Start the dashboard

Run the local chain, deploy the token, and start the Rust backend first. Then:

```powershell
pnpm dev
```

Open `http://127.0.0.1:5173`. Vite proxies `/api` and `/health` to `http://127.0.0.1:3000`, so no development CORS configuration is required.

Expected behavior:

1. the page displays a loading skeleton while React Query requests the first cached observation;
2. if the backend is unavailable, the page displays a Polish error panel and retries the query;
3. after the HTTP bootstrap, the SSE badge changes to `SSE połączono`;
4. a new observation arrives approximately every ten seconds;
5. the chart appears after at least two live observations;
6. stopping the backend changes the SSE badge to `SSE rozłączono`;
7. the browser's `EventSource` automatically attempts to reconnect.

To make block activity visible, run `npm run traffic` in the token project as described in the root and token READMEs. Transfer traffic advances the block number. It does not change the supply chart after initial funding because an ERC-20 transfer preserves `totalSupply`.

Stop Vite with `Ctrl+C`.

## Verification

```powershell
pnpm test
pnpm build
```
