import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { AdminDashboard } from "@/features/dashboard/admin-dashboard"
import { WhitePaper } from "@/features/white-paper/white-paper"

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { refetchOnWindowFocus: true },
  },
})

export function App() {
  return <QueryClientProvider client={queryClient}>{window.location.pathname.startsWith("/white-paper")?<WhitePaper/>:<AdminDashboard/>}</QueryClientProvider>
}
