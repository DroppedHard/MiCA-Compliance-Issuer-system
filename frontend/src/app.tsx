import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { AdminDashboard } from "@/features/dashboard/admin-dashboard"

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { refetchOnWindowFocus: true },
  },
})

export function App() {
  return <QueryClientProvider client={queryClient}><AdminDashboard /></QueryClientProvider>
}
