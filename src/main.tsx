import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { StrictMode } from "react"
import { createRoot } from "react-dom/client"

import "./index.css"
import { ThemeProvider } from "@/components/theme-provider.tsx"
import App from "./App.tsx"

const queryClient = new QueryClient()

// Suppress WKWebView-default browser interactions that don't belong in a
// desktop app: right-click context menu and Cmd+R reload / Cmd+0/+/-/= zoom.
// Devtools remain reachable via Cmd+Option+I in dev builds.
window.addEventListener("contextmenu", (e) => e.preventDefault())
window.addEventListener("keydown", (e) => {
  if (!(e.metaKey || e.ctrlKey)) return
  const key = e.key.toLowerCase()
  if (key === "r" || key === "0" || key === "=" || key === "-" || key === "+") {
    e.preventDefault()
  }
})

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <App />
      </ThemeProvider>
    </QueryClientProvider>
  </StrictMode>
)
