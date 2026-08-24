import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { AdminDashboard } from "@/features/dashboard/admin-dashboard"
import { ClientPortal } from "@/features/client/client-portal"

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { refetchOnWindowFocus: true },
  },
})

export function App() {
  const view = resolveAppView(window.location.pathname)
  return <QueryClientProvider client={queryClient}>{view === "client" ? <ClientPortal /> : <AdminDashboard />}</QueryClientProvider>
}

export const resolveAppView = (pathname: string): "admin" | "client" =>
  pathname === "/client" || pathname.startsWith("/client/") ? "client" : "admin"
